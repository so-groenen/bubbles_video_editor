pub mod helper_function;
use helper_function::*;
pub use helper_function::get_video_name;
use opencv::videoio::VideoWriter;

mod data_structures;
pub use crate::backend::data_structures::*;

use opencv::prelude::*;
use opencv::{videoio::{self}, highgui, core::{rotate}};
use std::thread::{self};

const DEFAULT_FILE_NAME: &'static str = "Video Capture Edit";

trait ResetUpdate
{
    fn reset(&mut self);
}
impl ResetUpdate for bool
{
    fn reset(&mut self)
    {
        *self = false;
    }    
}

trait VideoRenderer
{
    fn pause(self: Box<Self>) -> Box<dyn VideoRenderer>;
    fn play(self: Box<Self>) -> Box<dyn VideoRenderer>;
    fn read_capture(&mut self, capture: &mut videoio::VideoCapture) -> Result<bool,opencv::Error>;
    fn update_frame_data(&mut self, workers: &mut WorkerThreadAsyncChannels);
    fn update_frame(&mut self)  -> Result<(),opencv::Error> ;
    fn update_window(&self, window: &mut HighGuiWindow)  -> Result<(),opencv::Error>;
    fn render(&mut self, window: &HighGuiWindow)  -> Result<(),opencv::Error>;
    fn send_progression(&self, workers: &mut WorkerThreadAsyncChannels);
    fn write(&self, writer: &mut Option<VideoWriter>)  -> Result<(), opencv::Error> ;
    fn update_frame_counter(&mut self);
    fn get_progression(&self) -> f32;
}

// NOTE: The PlayMode & PauseMode are very heavy, ideally one should use allocate on the stack using:
// let mut play_mode   = PlayMode::new();
// let mut pause_mode  = PauseMode::new();
// "&dyn VideoRenderer = &mut play_mode" 
// rather than using a smart pointer like "Box::new(dyn ..)", where we allocate new states every time
// Then use a transition function to pass data from "PlayMode" to "PauseMode"
struct PlayMode
{
    counter: usize,
    frame_count: usize,
    frame: Mat,
    result_frame: Mat,
    preview_frame: Mat,
    frame_sizes: FrameSizeManager,
}

impl PlayMode
{
    fn new(counter: usize, frame_count: usize, frame_sizes: FrameSizeManager) -> Self
    {
        Self 
        {
            counter,
            frame_count,
            frame: Mat::default(),
            result_frame: Mat::default(),
            // result_frame2: None,
            preview_frame: Mat::default(), 
            frame_sizes
        }
    }    
}
impl VideoRenderer for PlayMode
{
    fn pause(self: Box<Self>) -> Box<dyn VideoRenderer>
    {
        Box::new(PauseMode::new(self.counter, self.frame_count, self.frame_sizes, Some(self.frame)))
    }
    fn play(self: Box<Self>) -> Box<dyn VideoRenderer>
    {
        self
    }   
    fn read_capture(&mut self, capture: &mut videoio::VideoCapture) -> Result<bool, opencv::Error> 
    {
        capture.read(&mut self.frame)
    }
    fn send_progression(&self, worker_channels: &mut WorkerThreadAsyncChannels)
    {
        let progression = self.counter as f32 / self.frame_count as f32;
        worker_channels.send_progression(progression);
    }
    fn update_frame_data(&mut self, worker_channels: &mut WorkerThreadAsyncChannels) 
    {
        self.frame_sizes.update_from_main(worker_channels); 
    }
    fn update_frame(&mut self)  -> Result<(),opencv::Error> // we update every frame all the time, therefore resize everytime 
    {
        match self.frame_sizes.get_rotation()
        {
            Some(rotation) => {rotate(&self.frame, &mut self.result_frame, rotation.code())?;}
            None           => 
            {
                // Result is now stored in result_frame.
                opencv::core::swap(&mut self.frame, &mut self.result_frame)?;
            }
        }
        opencv::imgproc::resize(&self.result_frame, &mut self.preview_frame, self.frame_sizes.get_preview(), 0.,0., opencv::imgproc::INTER_LINEAR)?;
        Ok(())    
    }
    fn update_window(&self, window: &mut HighGuiWindow)  -> Result<(),opencv::Error> 
    {  
        window.resize(self.frame_sizes.get_preview())?;
        Ok(())
    }
    fn render(&mut self, window: &HighGuiWindow) -> Result<(),opencv::Error>
    {
        window.show(&self.preview_frame)?;
        Ok(())
    }
    fn write(&self, video_writer: &mut Option<VideoWriter>)  -> Result<(), opencv::Error>
    {
        if let Some(writer) = video_writer
        {
            writer.write(&self.result_frame)?;
        }
        Ok(())
    }
    fn update_frame_counter(&mut self) 
    {
        self.counter += 1;
    }    
    fn get_progression(&self) -> f32
    {
        let progression = self.counter as f32 / self.frame_count as f32;
        progression
    }   
}
struct PauseMode
{
    counter: usize,
    frame_count: usize,
    frame: Option<Mat>,             
    result_frame: Mat,               
    preview_frame: Mat,               
    frame_sizes: FrameSizeManager,
    should_rotate_frame: bool,
    should_rescale_frame: bool,
    should_rescale_gui: bool,
    should_update_window: bool,
}

impl PauseMode
{
    fn new(counter: usize, frame_count: usize, frame_sizes: FrameSizeManager, frame: Option<Mat>) -> Self
    {
        Self
        {   
            counter,
            frame_count,
            frame,
            result_frame: Mat::default(),
            preview_frame: Mat::default(),
            frame_sizes,
            should_rotate_frame: true,
            should_rescale_frame: true,
            should_rescale_gui: true,
            should_update_window: true,
        }
    }    
}
impl VideoRenderer for PauseMode
{
    fn update_frame_counter(&mut self) 
    {
    }    
    fn pause(self: Box<Self>) -> Box<dyn VideoRenderer>
    {
        self
    }
    fn play(self: Box<Self>) -> Box<dyn VideoRenderer>
    {
        Box::new(PlayMode::new(self.counter, self.frame_count, self.frame_sizes))
    }   
    fn render(&mut self, window: &HighGuiWindow) -> Result<(),opencv::Error>
    {
        window.show(&self.preview_frame)?;
        Ok(())
    }
    fn read_capture(&mut self, capture: &mut videoio::VideoCapture) -> Result<bool,opencv::Error> 
    {
        if self.frame.is_none()
        {
            let mut frame = Mat::default();
            let result    = capture.read(&mut frame);
            self.frame    = Some(frame);
            return result;
        }
        Ok(true)    
    }
    fn send_progression(&self, worker_channels: &mut WorkerThreadAsyncChannels)
    {
        let progression = self.counter as f32 / self.frame_count as f32;
        worker_channels.send_progression(progression);
    }
    fn update_frame_data(&mut self, worker_channels: &mut WorkerThreadAsyncChannels) 
    {
        if self.frame_sizes.update_flip(worker_channels)
        {
            self.should_rotate_frame = true;
        }
        if self.frame_sizes.update_gui_size(worker_channels)
        {
            self.should_rescale_gui = true;
        }
        if self.frame_sizes.update_rescale(worker_channels)
        {
            self.should_rescale_frame = true;
        }
        // "should_update_window" will be set to true on the first iteration
        // then it will be set to false when should_rescale_frame etc will be also set to false
        // "should_update_window" is true only if requested by a new update, which will rescale, rotatate etc...
        self.should_update_window = self.should_rescale_frame || self.should_rescale_gui || self.should_rotate_frame;
    }
    fn update_window(&self, window: &mut HighGuiWindow)  -> Result<(),opencv::Error> 
    {
        if self.should_update_window                         // is set by update_frame_data
        {
            window.resize(self.frame_sizes.get_preview())?;
        }
        Ok(())
    }
    fn update_frame(&mut self) -> Result<(),opencv::Error>  
    {                                                        
        if self.should_rotate_frame
        {
            match self.frame_sizes.get_rotation()
            {
                Some(rotation) => {rotate(&self.frame.as_ref().unwrap(), &mut self.result_frame, rotation.code())?;}
                None           => {self.result_frame = self.frame.as_ref().unwrap().clone();}
            } 
        }
        if self.should_rescale_gui || self.should_rotate_frame || self.should_rescale_frame
        {
            opencv::imgproc::resize(&self.result_frame, &mut self.preview_frame, self.frame_sizes.get_preview(), 0.,0., opencv::imgproc::INTER_LINEAR)?;
        }
        self.should_rescale_gui.reset();
        self.should_rescale_frame.reset();
        self.should_rotate_frame.reset();
        Ok(())
    }

    fn write(&self, _writer: &mut Option<VideoWriter>) -> Result<(), opencv::Error> 
    {
        Ok(())
    }
    fn get_progression(&self) -> f32
    {
        let progression = self.counter as f32 / self.frame_count as f32;
        progression
    }
}

 
pub fn process_video_thread(mut capture: videoio::VideoCapture, 
                            options: ProcessOptions,
                            thread_pool: &mut VideoProcThreadPool,
                            mut worker_channels: WorkerThreadAsyncChannels)
{
    let handle = thread::spawn(move ||
    {
        let video_info        = VideoInfo::new(&capture)?;
        let total_frame_count = video_info.frame_count;
 
        let frame_sizes   = FrameSizeManager::new(video_info.frame_size, options.flip, options.gui_scale, options.re_scale.unwrap_or(1_f32));
        let winname       = options.get_video_name(DEFAULT_FILE_NAME);
        let path_str      = options.get_edit_path_str();

        let mut window = HighGuiWindow::build(winname, highgui::WINDOW_AUTOSIZE)?;
 

        let mut video_writer: Option<VideoWriter> = None;
        if options.should_process
        {
            let fourcc   = VideoWriter::fourcc('m', 'p', '4', 'v')?;
            let writer   = VideoWriter::new(path_str, fourcc, video_info.fps, frame_sizes.get_edit(), true).expect("Failed init writer!");
            video_writer = Some(writer)
        };


        let counter = 0;
        let mut video_renderer: Box<dyn VideoRenderer> = Box::new(PlayMode::new(counter, total_frame_count, frame_sizes));

        while worker_channels.is_not_aborted() && window.is_open()
        {
            if let Some(new_mode) = worker_channels.get_updated_video_mode()
            {
                video_renderer = match new_mode 
                {
                    VideoModes::Pause => video_renderer.pause(),
                    VideoModes::Play  => video_renderer.play(),
                }
            }
            if !video_renderer.read_capture(&mut capture)? || highgui::wait_key(10)? > 0
            {
                break;
            }
            video_renderer.update_frame_data(&mut worker_channels);
            video_renderer.update_frame()?;
            video_renderer.update_window(&mut window)?;
            video_renderer.render(&window)?;
            video_renderer.write(&mut video_writer)?;
            video_renderer.update_frame_counter();
            video_renderer.send_progression(&mut worker_channels);
        }

        if let Some(mut writer) = video_writer.take()
        {
            println!("VideoWriter closed at {}%",100_f32 * video_renderer.get_progression());
            writer.release()?;
        }
        Ok(capture)
    });

    thread_pool.push(handle);
}

 