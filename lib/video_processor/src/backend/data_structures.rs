use opencv::core::{RotateFlags,Size_};
use opencv::prelude::*;
use opencv::{videoio::{self, VideoCapture}, highgui};
use std::sync::mpsc::{self};
use std::thread::{JoinHandle};
pub type VideoProcThreadPool = Vec<JoinHandle<Result<VideoCapture, opencv::Error>>>;
use std::sync::mpsc::SendError;
use std::ffi::OsString;
use crate::backend::helper_function::*;

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum VideoModes
{
    Play,
    Pause,
}

#[derive(Debug, Default)]
pub struct VideoInfo
{
    pub frame_size: opencv::core::Size,
    pub frame_count: usize,
    pub fourcc_codec: (char, char, char, char),
    pub fps: f64,
}

use super::helper_function::decode_fourcc;
impl VideoInfo
{
    pub fn new(
        capture: &VideoCapture) -> Result<Self, opencv::Error>
    {
        let frame_count  = capture.get(videoio::CAP_PROP_FRAME_COUNT)? as usize;
        let height       = capture.get(videoio::CAP_PROP_FRAME_HEIGHT)? as i32;
        let width        = capture.get(videoio::CAP_PROP_FRAME_WIDTH)? as i32;
        let fps          = capture.get(videoio::CAP_PROP_FPS)? as f64;
        let fourcc_codec = capture.get(videoio::CAP_PROP_FOURCC)? as u32;
        let fourcc_codec = decode_fourcc(fourcc_codec).expect("FourCC cannot be negative");
        let frame_size = opencv::core::Size 
        {
            width,
            height
        };

        Ok( VideoInfo{
            frame_size,
            frame_count,
            fourcc_codec,
            fps,
        })
    }
}
#[derive(Debug)]

pub struct ProcessOptions
{
    pub gui_scale: f32,
    pub edit_file_name: std::path::PathBuf,
    pub flip: Option<RotateFlags>,
    pub should_process: bool,
    pub preview: bool,
    pub re_scale: Option<f32>,
}

impl Default for ProcessOptions
{
    fn default() -> Self {
        ProcessOptions
        {
            gui_scale: 1_f32,
            edit_file_name: std::path::PathBuf::new(),
            flip: None,
            should_process: false,
            preview: true,
            re_scale: None,
        }
    }    
}
impl ProcessOptions
{
    pub fn get_edit_path_str(&self) -> &str
    {
        let mut path_str = self.edit_file_name.to_str().expect("Not empty");    
        if path_str.ends_with("\"") && path_str.starts_with("\"")
        {
            path_str = path_str.trim_matches('\"');
        }
        path_str
    }    
    pub fn get_video_name(&self, default: &str) -> String
    {
        let default   = OsString::from(default);
        let file_name = self.edit_file_name.file_name()
            .unwrap_or(&default)
            .to_str()
            .expect("Failed converting OsString to &str");
        String::from(file_name)
    }
}

#[derive(Debug)]
pub struct MainThreadAsyncChannels
{
    pub rx_progression_from_thread: mpsc::Receiver<f32>,
    pub tx_abort_signal_to_thread: mpsc::Sender<bool>,
    pub tx_video_mode: mpsc::Sender<VideoModes>,
    pub tx_flip_update: mpsc::Sender<Option<RotateFlags>>,
    pub tx_rescale_update: mpsc::Sender<f32>,
    // pub rx_open_status: mpsc::Receiver<bool>,        // Could be useful, maybe not?
    pub tx_highgui_size_update: mpsc::Sender<f32>,
}
impl MainThreadAsyncChannels
{
    pub fn get_last_progression(&self) -> Option<f32>
    {
        self.rx_progression_from_thread.try_iter().last()
    }
    pub fn send_abort_signal(&self) -> Result<(), SendError<bool>>
    {
        self.tx_abort_signal_to_thread.send(true)?;
        Ok(())
    }
    // pub fn get_open_status(&self) -> Option<bool>  // Could be useful, maybe not?
    // {
    //     self.rx_open_status.try_iter().last()
    // }
    pub fn send_new_video_mode(&self, video_mode: VideoModes) -> Result<(), SendError<VideoModes>>
    {
        self.tx_video_mode.send(video_mode)?;
        Ok(())
    }
    pub fn send_new_gui_size(&self, new_gui_size: f32) -> Result<(), SendError<f32>>
    {
        self.tx_highgui_size_update.send(new_gui_size)?;
        Ok(())
    }
    pub fn send_new_rescale(&self, new_rescale: f32) -> Result<(), SendError<f32>>
    {
        self.tx_rescale_update.send(new_rescale)?;
        Ok(())
    }
    pub fn send_new_flip(&self, new_flip: Option<RotateFlags>) -> Result<(), SendError<Option<RotateFlags>>>
    {
        self.tx_flip_update.send(new_flip)?;
        Ok(())
    }
}

#[derive(Debug)]

pub struct WorkerThreadAsyncChannels
{
    pub tx_progression_to_main: mpsc::Sender<f32>,
    pub rx_abort_signal_from_main: mpsc::Receiver<bool>,
    pub rx_video_mode: mpsc::Receiver<VideoModes>,
    pub rx_flip_update: mpsc::Receiver<Option<RotateFlags>>,
    pub rx_rescale_update: mpsc::Receiver<f32>,
    // pub tx_open_status: mpsc::Sender<bool>,              // Could be useful, maybe not?
    pub rx_highgui_size_update: mpsc::Receiver<f32>,
}

impl WorkerThreadAsyncChannels
{
    pub fn get_updated_video_mode(&mut self) -> Option<VideoModes>
    {
        self.rx_video_mode.try_iter().last()
    }   
    pub fn get_last_size_update(&mut self) -> Option<f32>
    {
        self.rx_highgui_size_update.try_iter().last()
    }    
    pub fn get_rescale_update(&mut self) -> Option<f32>
    {
        self.rx_rescale_update.try_iter().last()
    }    
    pub fn get_updated_flip(&mut self) -> Option<Option<RotateFlags>>
    {
        self.rx_flip_update.try_iter().last()
    }    
    pub fn send_progression(&self, progression: f32)
    {
        self.tx_progression_to_main.send(progression).expect("Failed sending progression to main!");
    }
    pub fn is_not_aborted(&self) -> bool
    {
        self.rx_abort_signal_from_main.try_recv().is_err()
    }
    // pub fn send_open_status(&self, status: bool)     // Could be useful, maybe not?
    // {
    //     self.tx_open_status.send(status).expect("Failed sending progression to main!");
    // }
}

 

pub struct FrameSizeManager
{
    frame_size: Size_<i32>,
    rescaled_frame_size: Size_<i32>,
    rotated_rescaled_frame_size: Size_<i32>,
    preview_frame_size: Size_<i32>,
    gui_scale: f32,
    re_scale: f32,
    rotation: Option<RotateFlags>
}
impl FrameSizeManager
{
    pub fn new(frame_size: Size_<i32>, rotation: Option<RotateFlags>, gui_scale: f32, re_scale: f32) -> Self
    {
        let mut new_sizes = FrameSizeManager 
        { 
            frame_size,
            rescaled_frame_size:         frame_size, 
            rotated_rescaled_frame_size: frame_size, 
            preview_frame_size:          frame_size, 
            gui_scale,
            re_scale,
            rotation
        };
        new_sizes.resize_frame(re_scale); // will also rotate + rescale preview!
        new_sizes
    }
    pub fn resize_frame(&mut self, new_rescale: f32)
    {
        self.re_scale            = new_rescale;
        self.rescaled_frame_size = self.frame_size.get_resized(self.re_scale);
        self.rotate(self.rotation);
    }   
    pub fn rotate(&mut self, flip: Option<RotateFlags>)
    {
        self.rotation                    = flip;
        self.rotated_rescaled_frame_size = self.rescaled_frame_size.get_rotated(flip);
        self.preview_frame_size          = self.rotated_rescaled_frame_size.get_resized(self.gui_scale);
    }     
    pub fn resize_gui(&mut self, new_gui_scale: f32)
    {
        self.gui_scale          = new_gui_scale;
        self.preview_frame_size = self.rotated_rescaled_frame_size.get_resized(new_gui_scale);
    }    
    pub fn get_preview(&self) -> Size_<i32>
    {
        self.preview_frame_size
    }
    pub fn get_edit(&self) -> Size_<i32>
    {
        self.rotated_rescaled_frame_size
    }
    pub fn get_rotation(&self) -> Option<RotateFlags>
    {
        self.rotation
    }
    pub fn update_gui_size(&mut self, worker_channels: &mut WorkerThreadAsyncChannels) -> bool
    {
 
        if let Some(new_gui_scale) = worker_channels.get_last_size_update()
        {
            self.resize_gui(new_gui_scale);
            return true;
        }            
        false
    }
    pub fn update_flip(&mut self, worker_channels: &mut WorkerThreadAsyncChannels) -> bool
    {
 
        if let Some(new_flip) = worker_channels.get_updated_flip()
        {
            self.rotate(new_flip);
            return true
        }       
        false
    }
    pub fn update_rescale(&mut self, worker_channels: &mut WorkerThreadAsyncChannels) -> bool
    {
 
        if let Some(new_scale) = worker_channels.get_rescale_update()
        {
            self.resize_frame(new_scale);
            return true;
        }       
        false
    }
    pub fn update_from_main(&mut self, worker_channels: &mut WorkerThreadAsyncChannels) -> bool
    {
        self.update_flip(worker_channels) || self.update_gui_size(worker_channels) || self.update_rescale(worker_channels)
    }
}


pub struct HighGuiWindow
{
    winname: String,
}
impl HighGuiWindow
{
    pub fn build(winname: String, mode: i32) -> Result<HighGuiWindow,opencv::Error>
    {
        highgui::named_window(&winname[..], mode)?;
        Ok(Self { winname })
    }    
    pub fn show(&self,  mat: &impl opencv::core::ToInputArray) -> Result<(),opencv::Error>
    {
        highgui::imshow(self.winname.as_str(), mat)?; 
        Ok(())
    }
    pub fn resize(&mut self, size: opencv::core::Size)-> Result<(),opencv::Error>
    {
        highgui::resize_window_size(self.winname.as_str(), size)?;
        Ok(())
    }
    // Trick from https://stackoverflow.com/a/9321629
    pub fn is_open(&self) -> bool
    {
        highgui::get_window_property(self.winname.as_str(), highgui::WindowPropertyFlags::WND_PROP_FULLSCREEN.into()).is_ok()
    }
}
impl Drop for HighGuiWindow
{
    fn drop(&mut self) 
    {
        if self.is_open()
        {
            let _ = highgui::destroy_window(self.winname.as_str());
        }
    }    
}