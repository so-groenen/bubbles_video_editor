mod backend;

use crate::backend::{MyThreadPool2};
use crate::backend::{MainThreadAsyncChannels, MainThreadAsyncChannelsNOPT, WorkerThreadAsyncChannels, ResetChannels};
use crate::backend::load_video_from_file;
use crate::backend::process_video_thread;
pub use crate::backend::VideoInfo;
pub use crate::backend::ProcessOptions;
pub use crate::backend::get_video_name;

use std::sync::mpsc;

use opencv::prelude::*;
use opencv::{videoio, Result};
pub use opencv::core::RotateFlags;

const RESET_PROGRESS: f32 = 0f32;

use std::sync::mpsc::SendError;
pub use backend::helper_function::decode_fourcc;
#[derive(Debug)]
pub struct VideoProcessor
{
    // high_gui_size_ratio: i32,
    high_gui_scale: f32,
    file_name: std::path::PathBuf,
    // thread_pool: MyThreadPool,
    // progression: f64,
    thread_pool: MyThreadPool2,
    my_video: Option<videoio::VideoCapture>,
    // edited_video: Option<videoio::VideoCapture>,
    // main_thread_async_channels: MainThreadAsyncChannels, // options(MainThreadAsyncChannels)??
    main_async_channels: Option<MainThreadAsyncChannelsNOPT>,
    my_flip: Option<RotateFlags>,
    is_video_loaded: bool,
    pub video_info: Option<VideoInfo>,
}

impl Default for VideoProcessor 
{
    fn default() -> Self
    {
        VideoProcessor
        {
            // high_gui_size_ratio: 4i32,
            high_gui_scale: 0.25f32,
            file_name: std::path::PathBuf::new(),
            // progression: 0f64,
            thread_pool: MyThreadPool2::default(),
            my_video: None,
            // edited_video: None,
            // main_thread_async_channels: MainThreadAsyncChannels::default(),
            main_async_channels: None,
            my_flip: None,
            is_video_loaded: false,
            video_info: None // We do not need the filename, we can let the GUI handle this
        }
    }
}

impl VideoProcessor
{
    pub fn resize_gui(&self, scale: f32) //-> Result<(), SendError<f32>>
    {
        // if let Some(tx_gui_size_update) =  &self.main_thread_async_channels.tx_highgui_size_update
        // {
        //     tx_gui_size_update.send(scale)?
        // }
        // Ok(())

        self.main_async_channels
            .as_ref()
            .inspect(|channels|
            {
                channels.send_new_gui_size(scale).expect("Could not send resize gui");
            });
    }
    pub fn get_current_info(&self) -> Option<VideoInfo>
    {
        self.my_video
            .as_ref()
            .and_then(|capture|
            {
                Some(VideoInfo::new(capture).expect("Cannot construct VideoInfo: OpenCv error"))
            })
        // if let Some(capture) = &self.my_video
        // {
        //     // let default = OsString::from("Video Capture");
        //     // let name = self.file_name.file_name()
        //     //     .unwrap_or(&default)
        //     //     .to_str()
        //     //     .expect("Failed converting OsString to &str");
        //     let info: VideoInfo = VideoInfo::new(capture).expect("Cannot construct VideoInfo: OpenCv error");
        //     return Some(info);
        // }
        // None
    }
    pub fn unload_video(&mut self) -> Result<bool, opencv::Error>
    {
        if let Some(mut vid) = self.my_video.take()
        {
            vid.release()?;
            self.video_info      = None;
            self.is_video_loaded = false;
            return Ok(true);
        }
        Ok(false)
    }
    pub fn is_video_loaded(&self) -> bool
    {
        self.is_video_loaded
    }
    pub fn set_gui_scale(&mut self, scale: f32) -> Result<(), SendError<f32>>
    {
        if self.high_gui_scale != scale && self.has_launched_process()
        {
            self.high_gui_scale = scale;
            // return 
            self.resize_gui(scale);
        }
        self.high_gui_scale = scale;
        Ok(())
    }
    // pub fn create_process_option(&self, flip: Option<RotateFlags>) -> ProcessOptions
    // {
    //     ProcessOptions
    //     { 

    //         flip,
    //         // actions,
    //         should_process: true,
    //         preview: true,
    //         re_scale: None,
    //         output_format: self.get_current_info(),
    //     }
    // }
    pub fn set_flip(&mut self, flip: Option<RotateFlags>)
    {
        self.my_flip = flip; 
    }
    pub fn try_grab_video(&mut self, file_name: &std::path::PathBuf) -> bool
    {
        self.my_video  = load_video_from_file(&file_name);
        println!("try_grab_video: {}", file_name.to_str().expect("try_grab_video: path-to-str Conversion error"));
        self.file_name = file_name.clone();
        if self.has_video()
        {
            println!("try_grab_video: Success, we have the video!");
            self.is_video_loaded = true;
            self.video_info = self.get_current_info();
        }
        self.is_video_loaded
    }
    pub fn has_video(&self) -> bool
    {
        self.my_video.is_some()

        // match &self.my_video
        // {
        //     Some(_) => true,
        //     None => false,
        // }
    }
    pub fn dispatch_video_process(&mut self, options: ProcessOptions) -> bool
    {
        if let Some(capture) = self.my_video.take()
        {
            let (tx_progression_to_main, rx_progression_from_thread) = mpsc::channel();
            let (tx_abort_signal_to_thread, rx_abort_signal_from_main) = mpsc::channel();
            let (tx_open_status, rx_open_status) = mpsc::channel();
            let (tx_highgui_size_update, rx_highgui_size_update) = mpsc::channel();

            let main_channels = MainThreadAsyncChannelsNOPT
            {
                rx_progression_from_thread,
                tx_abort_signal_to_thread,
                rx_open_status,
                tx_highgui_size_update,
            };

            // self.main_thread_async_channels.rx_progression_from_thread = Some(rx_progression_from_thread);
            // self.main_thread_async_channels.tx_abort_signal_to_thread  = Some(tx_abort_signal_to_thread);
            // self.main_thread_async_channels.rx_opening_failure         = Some(rx_open_status);
            // self.main_thread_async_channels.tx_highgui_size_update     = Some(tx_highgui_size_update);
       

            let worker_channels = WorkerThreadAsyncChannels
            {
                tx_progression_to_main,
                rx_abort_signal_from_main,
                tx_open_status,
                rx_highgui_size_update, 
            };

            self.main_async_channels = Some(main_channels);

            process_video_thread(capture, options,  &mut self.thread_pool, worker_channels);


            println!(">> App (Main): Move Resouces [video] to worker thread...");
            // dispatch_preview_video(capture, self.my_flip, &mut self.thread_pool, &mut self.main_thread_async_channels);
            // dispatch_preview_video(capture, options, &mut self.thread_pool, &mut self.main_thread_async_channels);
            return true;
        }
        false
    }
    pub fn try_abort(&mut self) -> bool //Result<bool, SendError<bool>>
    {
        self.main_async_channels
            .as_ref()
            .is_some_and(|channels| 
            {
                channels.send_abort_signal().is_ok()
            })

        // self.main_async_channels
        //     .as_ref()
        //     .err
        // // if let Some(tx_signal) = &self.main_thread_async_channels.tx_abort_signal_to_thread
        // // {
        // //     tx_signal.send(true)?;
        // //     return Ok(true);
        // // }
        // // Ok(false)
    }
    pub fn get_progression(&self) -> Option<f32>
    {
        self.main_async_channels
            .as_ref()
            .and_then(|channels|
            {
                channels.get_last_progression()
            })


        // if let Some(prog_reciever) = &self.main_thread_async_channels.rx_progression_from_thread  
        // {
        //     return prog_reciever.try_iter().last()
        // }
        // None

        // self.main_thread_async_channels.rx_progression_from_thread
        //     .as_ref()
        //     .and_then(|prog_reciever|
        //     {
        //         prog_reciever.try_iter().last()
        //     })

    }
    pub fn has_launched_process(&self) -> bool
    {
        !self.thread_pool.is_empty()
    }
    pub fn is_process_finished(&self) -> bool
    {
        self.thread_pool.first()
            .is_some_and(|thread|
            {
                thread.is_finished()
            }) 
        // match self.thread_pool.first()  
        // {
        //     Some(thread) => thread.is_finished(),
        //     None => false,
        // }
        
    }
    // fn join_thread(&mut self) -> Result<(), opencv::Error>
    // {
    //     if self.has_launched_process()
    //     {
    //         let thread = self.thread_pool.pop().expect("Threadpool empty.");
    //         let mut original_video = thread.join().expect("Failed joining thread!")?;
    //         let final_progress = original_video.get(videoio::CAP_PROP_POS_AVI_RATIO)? as f32;

    //         original_video.set(videoio::CAP_PROP_POS_AVI_RATIO, 0.)?; // Reset frame count to 0
    //         self.my_video = Some(original_video);//.take();
    //         println!(">> App (join_thread): Resource returned successfully!");    
    //     }
    //     Ok(final_progress)
    // }
    pub fn handle_thread_join(&mut self)  -> Result<f32, opencv::Error>
    {
        // this should be strings to look for errors
        // if let Some(rx_video_from_thread) = self.main_thread_async_channels.rx_video_from_thread.take() && let Ok(mut edited_video) = rx_video_from_thread.try_recv()
        // {
        //     println!(">> App (Main): workerThread joined, recieved async resource: {:?}", edited_video);
        //     edited_video.set(videoio::CAP_PROP_POS_AVI_RATIO, 0.)?; // Reset frame count to 0
        //     self.edited_video = Some(edited_video).take();
        // };
        // if let Some(rx_progression_from_thread) = self.main_thread_async_channels.rx_progression_from_thread.take() && let Ok(msg) = rx_progression_from_thread.try_recv()
        // {
        //     println!(">> App (Main): Got msg from worker: \"{msg}\"");
        // };
        let mut final_progress = RESET_PROGRESS;
        if self.has_launched_process()
        {
            let thread = self.thread_pool.pop().expect("Threadpool empty.");
            let mut original_video = thread.join().expect("Failed joining thread!")?;
            
            let current_frame = original_video.get(videoio::CAP_PROP_POS_FRAMES)? as f32;
            let frame_count   = original_video.get(videoio::CAP_PROP_FRAME_COUNT)? as f32;
            final_progress    = current_frame/frame_count;

            original_video.set(videoio::CAP_PROP_POS_AVI_RATIO, 0.)?; // Reset frame count to 0
            self.my_video = Some(original_video);//.take();
            println!(">> App (join_thread): Resource returned successfully!");    
        }

        self.main_async_channels = None;
        // self.main_thread_async_channels.reset(); // This should be put to None if we use options
        // self.join_thread()?;
        Ok(final_progress)
    }
    pub fn clean_up(&mut self) -> Result<(), opencv::Error>
    {
        // println!(">> App: Cleanup: ");
        if self.has_launched_process()
        {
            println!(">> App (cleanup): !!!! Thread still running: Abort!");    
            if self.try_abort()
            {
                println!(">> App (cleanup): !!!! Abort msg Successfully sent");
            }
            else
            {
                panic!("!!!! No transmitter present / Could not send abort signal!");
            }

            // match self.try_abort() // should be option?
            // {
            //     Ok(has_transmitter) =>
            //     {
            //         if has_transmitter
            //         {
            //             println!(">> App (cleanup): !!!! Abort msg Successfully sent");
            //         }
            //         else
            //         {
            //             panic!("!!!! No transmitter present to send abort signal!");
            //         }
            //     }
            //     Err(e) => panic!("!!!! Could not abort thread {e}"),
            // }

            // let timeout = Duration::from_millis(100);
            // remove this
            // if let Some(rx_video_from_thread) = self.main_thread_async_channels.rx_video_from_thread.take() && let Ok(mut edited_video) = rx_video_from_thread.recv_timeout(timeout)
            // {
            //     println!(">> App (cleanup): Thread joined, recieved async resource: {:?}", edited_video);
            //     edited_video.set(videoio::CAP_PROP_POS_AVI_RATIO, 0.)?; // Reset frame count to 0
            //     self.edited_video = Some(edited_video).take();
            // };
            self.handle_thread_join()?;
        }

        if let Some(mut original_vid) = self.my_video.take()
        {
            println!(">> App (cleanup): Original video released");
            original_vid.release()?
        }
        // if let Some(mut edited_vid) = self.edited_video.take()
        // {
        //     edited_vid.release()?
        // }
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
