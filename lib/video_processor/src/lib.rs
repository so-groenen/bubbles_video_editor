mod backend;

use crate::backend::VideoModes;
pub use crate::backend::get_video_name;
use crate::backend::process_video_thread;
use crate::backend::VideoProcThreadPool;
pub use crate::backend::ProcessOptions;
pub use crate::backend::VideoInfo;
use crate::backend::{MainThreadAsyncChannels, WorkerThreadAsyncChannels};

use std::sync::mpsc;

pub use opencv::core::RotateFlags;
use opencv::prelude::*;
use opencv::{videoio, Result};

const RESET_PROGRESS:    f32   = 0_f32;
const GUI_DEFAULT_SCALE: f32   = 1_f32;
const FRAME_DEFAULT_SCALE: f32 = 1_f32;

pub use backend::helper_function::decode_fourcc;
use std::sync::mpsc::SendError;
#[derive(Debug)]
pub struct VideoProcessor 
{
    high_gui_scale: f32,
    re_scale: f32,
    file_name: std::path::PathBuf,
    thread_pool: VideoProcThreadPool,
    my_video: Option<videoio::VideoCapture>,
    main_async_channels: Option<MainThreadAsyncChannels>,
    my_flip: Option<RotateFlags>,
    video_mode: Option<VideoModes>,
    pub video_info: Option<VideoInfo>,
}

impl Default for VideoProcessor 
{
    fn default() -> Self
    {
        VideoProcessor
        {
            high_gui_scale: GUI_DEFAULT_SCALE,
            re_scale: FRAME_DEFAULT_SCALE,
            file_name: std::path::PathBuf::new(),
            thread_pool: VideoProcThreadPool::default(),
            my_video: None,
            main_async_channels: None,
            my_flip: None,
            video_mode: None,
            video_info: None, // We do not need the filename, we can let the GUI handle this
        }
    }
}

impl VideoProcessor 
{
    fn send_new_gui_size(&self, scale: f32) //-> Result<(), SendError<f32>>
    {
        self.main_async_channels.as_ref().inspect(|channels|
        {
            channels.send_new_gui_size(scale)
                    .expect("Could not send resize gui");
        });
    }
    fn send_new_flip(&self, flip: Option<RotateFlags>) //-> Result<(), SendError<f32>>
    {
        self.main_async_channels.as_ref().inspect(|channels|
        {
            channels.send_new_flip(flip)
                    .expect("Could not send flip");
        });
    }
    fn send_video_mode(&self, video_mode: VideoModes) //-> Result<(), SendError<f32>>
    {
        self.main_async_channels.as_ref().inspect(|channels|
        {
            channels.send_new_video_mode(video_mode)
                    .expect("Could not send video_mode");
        });
    }
    fn send_rescale(&self, rescale: f32) //-> Result<(), SendError<f32>>
    {
        self.main_async_channels.as_ref().inspect(|channels|
        {
            channels.send_new_rescale(rescale)
                    .expect("Could not send rescale");
        });
    }
    pub fn get_current_info(&self) -> Option<VideoInfo> 
    {
        self.my_video.as_ref().and_then(|capture| 
        {
            Some(VideoInfo::new(capture).expect("Cannot construct VideoInfo: OpenCv error"))
        })
    }
    pub fn unload_video(&mut self) -> Result<bool, opencv::Error>
    {
        if let Some(mut vid) = self.my_video.take() 
        {
            vid.release()?;
            self.video_info = None;
            return Ok(true);
        }
        Ok(false)
    }
    pub fn set_gui_scale(&mut self, scale: f32) -> Result<(), SendError<f32>> 
    {
        if self.high_gui_scale != scale && self.has_launched_process() 
        {
            self.send_new_gui_size(scale);
        }
        self.high_gui_scale = scale;
        Ok(())
    }
    pub fn set_rescale(&mut self, rescale: f32) -> Result<(), SendError<f32>> 
    {
        if self.re_scale != rescale && self.has_launched_process() 
        {
            self.send_rescale(rescale);
        }
        self.re_scale = rescale;
        Ok(())
    }
    pub fn pause_video(&mut self) -> Result<(), SendError<VideoModes>> 
    {
        if self.has_launched_process() && self.video_mode.as_ref().is_some_and(|mode| *mode == VideoModes::Play) 
        {
            let pause = VideoModes::Pause;
            self.send_video_mode(pause);
            self.video_mode = Some(pause);
        }
        Ok(())
    }
    pub fn resume_video(&mut self) -> Result<(), SendError<VideoModes>> 
    {
        if self.has_launched_process() && self.video_mode.as_ref().is_some_and(|mode| *mode == VideoModes::Pause) 
        {
            let play = VideoModes::Play;
            self.send_video_mode(play);
            self.video_mode = Some(play);
        }
        Ok(())
    }
    pub fn set_flip(&mut self, flip: Option<RotateFlags>) -> Result<(), SendError<RotateFlags>> 
    {
        if self.my_flip != flip && self.has_launched_process() 
        {
            self.send_new_flip(flip);
        }
        self.my_flip = flip;
        Ok(())
    }

    pub fn try_grab_video(&mut self, file_name: &std::path::PathBuf) -> bool
    {
        self.my_video = load_video_from_file(&file_name);
        println!("try_grab_video: {}", file_name
                .to_str()
                .expect("try_grab_video: path-to-str Conversion error")
        );
        self.file_name = file_name.clone();
        if self.has_video() 
        {
            println!("try_grab_video: Success, we have the video!");
            self.video_info = self.get_current_info();
        }
        self.has_video()
    }
    pub fn has_video(&self) -> bool 
    {
        self.my_video.is_some() || self.has_launched_process()
    }
    pub fn dispatch_video_process(&mut self, options: ProcessOptions) -> bool
    {
        if let Some(capture) = self.my_video.take() 
        {
            let (tx_progression_to_main,    rx_progression_from_thread) = mpsc::channel();
            let (tx_abort_signal_to_thread, rx_abort_signal_from_main)  = mpsc::channel();
            let (tx_video_mode,             rx_video_mode)              = mpsc::channel();
            let (tx_flip_update,            rx_flip_update)             = mpsc::channel();
            let (tx_rescale_update,         rx_rescale_update)          = mpsc::channel();
            // let (tx_open_status,            rx_open_status)             = mpsc::channel();
            let (tx_highgui_size_update,    rx_highgui_size_update)     = mpsc::channel();

            let main_channels = MainThreadAsyncChannels 
            {
                rx_progression_from_thread,
                tx_abort_signal_to_thread,
                tx_video_mode,
                tx_flip_update,
                tx_rescale_update,
                // rx_open_status,
                tx_highgui_size_update,
            };

            let worker_channels = WorkerThreadAsyncChannels 
            {
                tx_progression_to_main,
                rx_abort_signal_from_main,
                rx_video_mode,
                rx_flip_update,
                rx_rescale_update,
                // tx_open_status,
                rx_highgui_size_update,
            };

            self.main_async_channels = Some(main_channels);
            self.video_mode          = Some(VideoModes::Play);
            process_video_thread(capture, options, &mut self.thread_pool, worker_channels);

            println!(">> App (Main): Move Resouces [video] to worker thread...");
            return true;
        }
        false
    }
    pub fn try_abort(&mut self) -> bool //Result<bool, SendError<bool>>
    {
        self.main_async_channels
            .as_ref()
            .is_some_and(|channels| channels.send_abort_signal().is_ok())

    }
    pub fn get_progression(&self) -> Option<f32> 
    {
        self.main_async_channels
            .as_ref()
            .and_then(|channels| channels.get_last_progression())
    }
    pub fn has_launched_process(&self) -> bool 
    {
        !self.thread_pool.is_empty()
    }
    pub fn is_process_finished(&self) -> bool
    {
        self.thread_pool
            .first()
            .is_some_and(|thread| thread.is_finished())
    }

    pub fn handle_thread_join(&mut self) -> Result<f32, opencv::Error> 
    {
        let mut final_progress = RESET_PROGRESS;
        if self.has_launched_process() 
        {
            let thread             = self.thread_pool.pop().expect("Threadpool empty.");
            let mut original_video = thread.join().expect("Failed joining thread!")?;

            let current_frame = original_video.get(videoio::CAP_PROP_POS_FRAMES)? as f32;
            let frame_count   = original_video.get(videoio::CAP_PROP_FRAME_COUNT)? as f32;
            final_progress    = current_frame / frame_count;

            original_video.set(videoio::CAP_PROP_POS_AVI_RATIO, 0.)?; // Reset frame count to 0
            self.my_video = Some(original_video); //.take();
            println!(">> App (join_thread): Resource returned successfully!");
        }
        self.video_mode          = None;
        self.main_async_channels = None;
        Ok(final_progress)
    }
    pub fn clean_up(&mut self) -> Result<(), opencv::Error> 
    {
        // println!(">> App: Cleanup: ");
        if self.has_launched_process()
        {
            println!(">> App (cleanup): !!!! Thread still running: Abort!");
            if self.try_abort() {
                println!(">> App (cleanup): !!!! Abort msg Successfully sent");
            } else {
                panic!("!!!! No transmitter present / Could not send abort signal!");
            }
            self.handle_thread_join()?;
        }

        if let Some(mut original_vid) = self.my_video.take()
        {
            println!(">> App (cleanup): Original video released");
            original_vid.release()?
        }
        Ok(())
    }
}

impl Drop for VideoProcessor
{
    fn drop(&mut self) 
    {
        println!("cleanup called");
        self.clean_up().expect("Video not released correctly");
    }
}

 pub fn load_video_from_file(file_path: &std::path::PathBuf) -> Option<videoio::VideoCapture>
{

    if let Some(file_path) = file_path.to_str() && !file_path.is_empty() // empty string -> passes "0" to c++ API -> uses webcam
    {
        let video =  videoio::VideoCapture::from_file(file_path, videoio::CAP_ANY).expect("OpenCv Binding error: Failed init video");
        if video.is_opened().expect("OpenCv Binding error: Cannot check if video is open or not")
        {
           return Some(video);
        }
    }
    None
}