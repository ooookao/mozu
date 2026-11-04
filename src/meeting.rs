use std::{
    ffi::c_void,
    mem::size_of,
    ptr, slice,
    sync::{
        Arc,
        atomic::{AtomicU8, Ordering},
    },
};

use core_foundation::{
    array::CFArray,
    base::{CFType, TCFType},
    dictionary::CFDictionary,
    number::CFNumber,
    string::{CFString, CFStringRef},
};
use coreaudio_sys as ca;
use mozu_audio::{Error, Result};
use objc2::{
    msg_send,
    rc::{Retained, autoreleasepool},
    runtime::{AnyClass, AnyObject, Bool, NSObject},
};
use objc2_foundation::{NSArray, NSString};
use rtrb::{Consumer, Producer, RingBuffer};

const CAPTURE_OK: u8 = 0;
const CAPTURE_FORMAT_CHANGED: u8 = 1;
const CAPTURE_OVERRUN: u8 = 2;

type CreateTap = unsafe extern "C" fn(*const AnyObject, *mut ca::AudioObjectID) -> ca::OSStatus;
type DestroyTap = unsafe extern "C" fn(ca::AudioObjectID) -> ca::OSStatus;

pub(crate) struct MeetingRecording {
    device: AggregateDevice,
    samples: Consumer<f32>,
    condition: Arc<AtomicU8>,
    sample_rate: u32,
}

impl MeetingRecording {
    pub(crate) fn start() -> Result<Self> {
        let tap = Tap::new()?;
        let mut device = AggregateDevice::new(tap)?;
        let (sample_rate, channels) = input_format(device.id)?;
        let (producer, samples) = RingBuffer::new(sample_rate as usize * 3);
        let condition = Arc::new(AtomicU8::new(CAPTURE_OK));
        device.start(Capture {
            producer,
            condition: condition.clone(),
            channels,
        })?;
        Ok(Self {
            device,
            samples,
            condition,
            sample_rate,
        })
    }

    pub(crate) fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    pub(crate) fn drain_input(&mut self, maximum: usize) -> Result<Vec<f32>> {
        let condition = self.condition.load(Ordering::Acquire);
        match condition {
            CAPTURE_OK => Ok(std::iter::from_fn(|| self.samples.pop().ok())
                .take(maximum)
                .collect()),
            CAPTURE_FORMAT_CHANGED => Err(Error::Invalid(
                "録音中にマイクまたはシステム音声の形式が変わりました".into(),
            )),
            _ => Err(Error::Invalid(
                "会議音声の録音バッファがいっぱいになりました".into(),
            )),
        }
    }

    pub(crate) fn stop_input(mut self) -> Result<Vec<f32>> {
        self.device.stop()?;
        self.drain_input(usize::MAX)
    }
}

struct Tap {
    id: ca::AudioObjectID,
    destroy: DestroyTap,
}

impl Tap {
    fn new() -> Result<Self> {
        autoreleasepool(|_| {
            let unsupported =
                || Error::Invalid("システム音声の収録にはmacOS 14.2以降が必要です".into());
            let class = AnyClass::get(c"CATapDescription").ok_or_else(unsupported)?;
            let create = unsafe {
                libc::dlsym(
                    libc::RTLD_DEFAULT,
                    c"AudioHardwareCreateProcessTap".as_ptr(),
                )
            };
            let destroy = unsafe {
                libc::dlsym(
                    libc::RTLD_DEFAULT,
                    c"AudioHardwareDestroyProcessTap".as_ptr(),
                )
            };
            if create.is_null() || destroy.is_null() {
                return Err(unsupported());
            }
            let create: CreateTap = unsafe { std::mem::transmute(create) };
            let destroy: DestroyTap = unsafe { std::mem::transmute(destroy) };
            let exclusions = NSArray::<NSObject>::from_slice(&[]);
            let description: Retained<AnyObject> = unsafe {
                msg_send![msg_send![class, alloc], initMonoGlobalTapButExcludeProcesses: &*exclusions]
            };
            let name = NSString::from_str("Mozu System Audio");
            let mut id = 0;
            unsafe {
                let _: () = msg_send![&description, setName: &*name];
                let _: () = msg_send![&description, setPrivate: Bool::YES];
                check(
                    "AudioHardwareCreateProcessTap",
                    create(&*description, &mut id),
                )?;
            }
            Ok(Self { id, destroy })
        })
    }
}

impl Drop for Tap {
    fn drop(&mut self) {
        let status = unsafe { (self.destroy)(self.id) };
        if let Err(error) = check("AudioHardwareDestroyProcessTap", status) {
            eprintln!("{error}");
        }
    }
}

struct AggregateDevice {
    id: ca::AudioObjectID,
    io: ca::AudioDeviceIOProcID,
    capture: Option<Box<Capture>>,
    running: bool,
    _tap: Tap,
}

impl AggregateDevice {
    fn new(tap: Tap) -> Result<Self> {
        let microphone = property::<ca::AudioObjectID>(
            ca::kAudioObjectSystemObject,
            ca::kAudioHardwarePropertyDefaultInputDevice,
            ca::kAudioObjectPropertyScopeGlobal,
        )?;
        if microphone == ca::kAudioObjectUnknown {
            return Err(Error::Invalid("マイクが見つかりません".into()));
        }
        let microphone_uid = string_property(microphone, ca::kAudioDevicePropertyDeviceUID)?;
        let tap_uid = string_property(tap.id, ca::kAudioTapPropertyUID)?;
        let enabled = CFNumber::from(1i32).as_CFType();
        let microphone_list = CFArray::from_CFTypes(&[dictionary(&[(
            ca::kAudioSubDeviceUIDKey,
            microphone_uid.as_CFType(),
        )])]);
        let tap_list = CFArray::from_CFTypes(&[dictionary(&[
            (ca::kAudioSubTapUIDKey, tap_uid.as_CFType()),
            (ca::kAudioSubTapDriftCompensationKey, enabled.clone()),
        ])]);
        let uid = CFString::new(&format!("local.mozu-audio.meeting.{tap_uid}"));
        let spec = dictionary(&[
            (
                ca::kAudioAggregateDeviceNameKey,
                CFString::new("Mozu Meeting").as_CFType(),
            ),
            (ca::kAudioAggregateDeviceUIDKey, uid.as_CFType()),
            (ca::kAudioAggregateDeviceIsPrivateKey, enabled),
            (
                ca::kAudioAggregateDeviceMainSubDeviceKey,
                microphone_uid.as_CFType(),
            ),
            (
                ca::kAudioAggregateDeviceSubDeviceListKey,
                microphone_list.as_CFType(),
            ),
            (ca::kAudioAggregateDeviceTapListKey, tap_list.as_CFType()),
        ]);
        let mut id = 0;
        check("AudioHardwareCreateAggregateDevice", unsafe {
            ca::AudioHardwareCreateAggregateDevice(spec.as_concrete_TypeRef().cast(), &mut id)
        })?;
        Ok(Self {
            id,
            io: None,
            capture: None,
            running: false,
            _tap: tap,
        })
    }

    fn start(&mut self, capture: Capture) -> Result<()> {
        let mut capture = Box::new(capture);
        check("AudioDeviceCreateIOProcID", unsafe {
            ca::AudioDeviceCreateIOProcID(
                self.id,
                Some(capture_input),
                ptr::from_mut(&mut *capture).cast(),
                &mut self.io,
            )
        })?;
        self.capture = Some(capture);
        check("AudioDeviceStart", unsafe {
            ca::AudioDeviceStart(self.id, self.io)
        })?;
        self.running = true;
        Ok(())
    }

    fn stop(&mut self) -> Result<()> {
        if self.running {
            check("AudioDeviceStop", unsafe {
                ca::AudioDeviceStop(self.id, self.io)
            })?;
            self.running = false;
        }
        Ok(())
    }
}

impl Drop for AggregateDevice {
    fn drop(&mut self) {
        if let Err(error) = self.stop() {
            eprintln!("{error}");
        }
        let detach = match self.io {
            Some(_) => unsafe { ca::AudioDeviceDestroyIOProcID(self.id, self.io) },
            None => 0,
        };
        let destroy = unsafe { ca::AudioHardwareDestroyAggregateDevice(self.id) };
        if let Err(error) = check("AudioDeviceDestroyIOProcID", detach) {
            eprintln!("{error}");
        }
        if let Err(error) = check("AudioHardwareDestroyAggregateDevice", destroy) {
            eprintln!("{error}");
            if detach != 0
                && let Some(capture) = self.capture.take()
            {
                // The driver may still hold the callback pointer.
                Box::leak(capture);
            }
        }
    }
}

fn dictionary(entries: &[(&[u8], CFType)]) -> CFDictionary<CFString, CFType> {
    let values: Vec<_> = entries
        .iter()
        .map(|(key, value)| {
            let key = std::ffi::CStr::from_bytes_with_nul(key).expect("Core Audio key");
            (
                CFString::new(key.to_str().expect("ASCII Core Audio key")),
                value.clone(),
            )
        })
        .collect();
    CFDictionary::from_CFType_pairs(&values)
}

fn string_property(id: ca::AudioObjectID, selector: u32) -> Result<CFString> {
    let value = property::<CFStringRef>(id, selector, ca::kAudioObjectPropertyScopeGlobal)?;
    if value.is_null() {
        return Err(Error::Invalid(format!(
            "Core Audio returned a null UID (object={id}, selector={selector:#010x})"
        )));
    }
    Ok(unsafe { CFString::wrap_under_create_rule(value) })
}

fn property<T: Copy + Default>(id: ca::AudioObjectID, selector: u32, scope: u32) -> Result<T> {
    let mut value = T::default();
    let mut size = size_of::<T>() as u32;
    let address = address(selector, scope);
    let status = unsafe {
        ca::AudioObjectGetPropertyData(
            id,
            &address,
            0,
            ptr::null(),
            &mut size,
            ptr::from_mut(&mut value).cast(),
        )
    };
    if status == 0 && size == size_of::<T>() as u32 {
        Ok(value)
    } else {
        Err(Error::Invalid(format!(
            "Core Audio property failed (object={id}, selector={selector:#010x}, size={size}, OSStatus={status})"
        )))
    }
}

fn address(selector: u32, scope: u32) -> ca::AudioObjectPropertyAddress {
    ca::AudioObjectPropertyAddress {
        mSelector: selector,
        mScope: scope,
        mElement: ca::kAudioObjectPropertyElementMain,
    }
}

fn input_format(device: ca::AudioObjectID) -> Result<(u32, usize)> {
    let address = address(
        ca::kAudioDevicePropertyStreams,
        ca::kAudioDevicePropertyScopeInput,
    );
    let mut size = 0;
    check("AudioObjectGetPropertyDataSize(input streams)", unsafe {
        ca::AudioObjectGetPropertyDataSize(device, &address, 0, ptr::null(), &mut size)
    })?;
    let mut streams = vec![0u32; size as usize / size_of::<ca::AudioObjectID>()];
    check("AudioObjectGetPropertyData(input streams)", unsafe {
        ca::AudioObjectGetPropertyData(
            device,
            &address,
            0,
            ptr::null(),
            &mut size,
            streams.as_mut_ptr().cast(),
        )
    })?;
    streams.truncate(size as usize / size_of::<ca::AudioObjectID>());
    let rate: f64 = property(
        device,
        ca::kAudioDevicePropertyNominalSampleRate,
        ca::kAudioObjectPropertyScopeGlobal,
    )?;
    let mut channels = 0;
    for stream in streams {
        let format = property::<ca::AudioStreamBasicDescription>(
            stream,
            ca::kAudioStreamPropertyVirtualFormat,
            ca::kAudioObjectPropertyScopeGlobal,
        )?;
        let flags = format.mFormatFlags;
        let expected_flags = ca::kAudioFormatFlagIsFloat | ca::kAudioFormatFlagIsPacked;
        if format.mFormatID != ca::kAudioFormatLinearPCM
            || flags & expected_flags != expected_flags
            || flags & ca::kAudioFormatFlagIsBigEndian != 0
            || format.mBitsPerChannel != 32
            || format.mSampleRate != rate
        {
            return Err(Error::Invalid(format!(
                "Unsupported Core Audio input format: {format:?}"
            )));
        }
        channels += format.mChannelsPerFrame as usize;
    }
    if !(8_000.0..=192_000.0).contains(&rate) || !(2..=64).contains(&channels) {
        return Err(Error::Invalid(format!(
            "マイクとシステム音声を開始できませんでした（{rate} Hz、{channels} ch）"
        )));
    }
    Ok((rate as u32, channels))
}

fn check(operation: &str, status: ca::OSStatus) -> Result<()> {
    if status == 0 {
        Ok(())
    } else {
        Err(Error::Invalid(format!(
            "{operation} failed (OSStatus={status})"
        )))
    }
}

struct Capture {
    producer: Producer<f32>,
    condition: Arc<AtomicU8>,
    channels: usize,
}

unsafe extern "C" fn capture_input(
    _device: ca::AudioObjectID,
    _now: *const ca::AudioTimeStamp,
    input: *const ca::AudioBufferList,
    _input_time: *const ca::AudioTimeStamp,
    _output: *mut ca::AudioBufferList,
    _output_time: *const ca::AudioTimeStamp,
    context: *mut c_void,
) -> ca::OSStatus {
    // The IOProc owns this context until Core Audio stops calling it.
    let capture = unsafe { &mut *context.cast::<Capture>() };
    if input.is_null() || capture.condition.load(Ordering::Relaxed) != CAPTURE_OK {
        return 0;
    }
    let buffers = unsafe {
        slice::from_raw_parts(
            ptr::addr_of!((*input).mBuffers).cast::<ca::AudioBuffer>(),
            (*input).mNumberBuffers as usize,
        )
    };
    let result = unsafe { mix_input(buffers, capture.channels, &mut capture.producer) };
    if result != CAPTURE_OK {
        capture.condition.store(result, Ordering::Release);
    }
    0
}

/// The buffers must be Core Audio float PCM buffers valid for the duration of the call.
unsafe fn mix_input(
    buffers: &[ca::AudioBuffer],
    channels: usize,
    producer: &mut Producer<f32>,
) -> u8 {
    let mut frames = None;
    let mut actual_channels = 0;
    for buffer in buffers {
        if buffer.mNumberChannels == 0 {
            return CAPTURE_FORMAT_CHANGED;
        }
        let bytes_per_frame = buffer.mNumberChannels as usize * size_of::<f32>();
        let bytes = buffer.mDataByteSize as usize;
        if bytes == 0 {
            continue;
        }
        if buffer.mData.is_null() || !bytes.is_multiple_of(bytes_per_frame) {
            return CAPTURE_FORMAT_CHANGED;
        }
        let count = bytes / bytes_per_frame;
        if frames.is_some_and(|frames| frames != count) {
            return CAPTURE_FORMAT_CHANGED;
        }
        frames = Some(count);
        actual_channels += buffer.mNumberChannels as usize;
    }
    let Some(frames) = frames else {
        return CAPTURE_OK;
    };
    if actual_channels > channels {
        return CAPTURE_FORMAT_CHANGED;
    }
    if frames > producer.slots() {
        return CAPTURE_OVERRUN;
    }

    for frame in 0..frames {
        let mut sum = 0.0;
        for buffer in buffers.iter().filter(|buffer| buffer.mDataByteSize != 0) {
            let count = buffer.mNumberChannels as usize;
            let data = unsafe {
                slice::from_raw_parts(buffer.mData.cast::<f32>().add(frame * count), count)
            };
            sum += data.iter().sum::<f32>();
        }
        if producer
            .push((sum / actual_channels as f32).clamp(-1.0, 1.0))
            .is_err()
        {
            return CAPTURE_OVERRUN;
        }
    }
    CAPTURE_OK
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem::size_of_val;

    #[test]
    fn mixes_simultaneous_microphone_and_system_frames() {
        let mut microphone = [0.6f32, 0.0, 0.4, 0.2];
        let mut system = [0.0f32, 0.9];
        let buffers = [
            ca::AudioBuffer {
                mNumberChannels: 2,
                mDataByteSize: size_of_val(&microphone) as u32,
                mData: microphone.as_mut_ptr().cast(),
            },
            ca::AudioBuffer {
                mNumberChannels: 1,
                mDataByteSize: size_of_val(&system) as u32,
                mData: system.as_mut_ptr().cast(),
            },
        ];
        let (mut producer, mut consumer) = RingBuffer::new(2);
        assert_eq!(unsafe { mix_input(&buffers, 3, &mut producer) }, CAPTURE_OK);
        assert!((consumer.pop().unwrap() - 0.2).abs() < 1e-6);
        assert!((consumer.pop().unwrap() - 0.5).abs() < 1e-6);
        assert_eq!(
            unsafe { mix_input(&buffers, 2, &mut producer) },
            CAPTURE_FORMAT_CHANGED
        );
    }
}
