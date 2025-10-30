use opencv::core::RotateFlags;
use opencv::prelude::*;
use opencv::{videoio::{self, VideoCapture}};
use std::sync::mpsc::{self};
use std::thread::{JoinHandle};
pub type MyThreadPool2 = Vec<JoinHandle<Result<VideoCapture, opencv::Error>>>;
use std::sync::mpsc::SendError;


#[derive(Debug, Default)]
pub struct VideoInfo
{
    // pub file_name: String,
    pub frame_size: opencv::core::Size,
    pub frame_count: usize,
    pub fourcc_codec: (char, char, char, char),
    pub fps: f64,
}

use super::helper_function::decode_fourcc;
impl VideoInfo
{
    pub fn new(
        // name: &str,
        capture: &VideoCapture) -> Result<Self, opencv::Error>
    {
        let frame_count =  capture.get(videoio::CAP_PROP_FRAME_COUNT)? as usize;
        let fps   =  capture.get(videoio::CAP_PROP_FPS)? as f64;
        let fourcc_codec =  capture.get(videoio::CAP_PROP_FOURCC)? as u32;
        let height =  capture.get(videoio::CAP_PROP_FRAME_HEIGHT)? as i32;
        let width =  capture.get(videoio::CAP_PROP_FRAME_WIDTH)? as i32;
        let fourcc_codec = decode_fourcc(fourcc_codec).expect("FourCC cannot be negative");
        let frame_size = opencv::core::Size 
        {
            width,
            height
        };

        Ok( VideoInfo{
            // file_name: String::from(name),
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
            gui_scale: 1f32,
            edit_file_name: std::path::PathBuf::new(),
            flip: None,
            should_process: false,
            preview: true,
            re_scale: None,
        }
    }    
}


#[derive(Debug, Default)]
pub struct MainThreadAsyncChannels
{
    pub rx_progression_from_thread: Option<mpsc::Receiver<f32>>,
    pub tx_abort_signal_to_thread: Option<mpsc::Sender<bool>>,
    pub rx_opening_failure: Option<mpsc::Receiver<bool>>,
    pub tx_highgui_size_update: Option<mpsc::Sender<f32>>,
}

#[derive(Debug)]
pub struct MainThreadAsyncChannelsNOPT
{
    pub rx_progression_from_thread: mpsc::Receiver<f32>,
    pub tx_abort_signal_to_thread: mpsc::Sender<bool>,
    pub rx_open_status: mpsc::Receiver<bool>,
    pub tx_highgui_size_update: mpsc::Sender<f32>,
}
impl MainThreadAsyncChannelsNOPT
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
    pub fn get_open_status(&self) -> Option<bool>
    {
        self.rx_open_status.try_iter().last()
    }
    pub fn send_new_gui_size(&self, new_gui_size: f32) -> Result<(), SendError<f32>>
    {
        self.tx_highgui_size_update.send(new_gui_size)?;
        Ok(())
    }
}

#[derive(Debug)]

pub struct WorkerThreadAsyncChannels
{
    pub tx_progression_to_main: mpsc::Sender<f32>,
    pub rx_abort_signal_from_main: mpsc::Receiver<bool>,
    pub tx_open_status: mpsc::Sender<bool>,//Should be string?
    pub rx_highgui_size_update: mpsc::Receiver<f32>,
}

impl WorkerThreadAsyncChannels
{
    pub fn get_last_size_update(&mut self) -> Option<f32>
    {
        self.rx_highgui_size_update.try_iter().last()
    }    
    pub fn send_progression(&self, progression: f32)
    {
        self.tx_progression_to_main.send(progression).expect("Failed sending progression to main!");
    }
    pub fn is_not_aborted(&self) -> bool
    {
        self.rx_abort_signal_from_main.try_recv().is_err()
    }
    pub fn send_open_status(&self, status: bool)
    {
        self.tx_open_status.send(status).expect("Failed sending progression to main!");
    }
}

pub trait ResetChannels
{
    fn reset(&mut self);    
}

impl ResetChannels for MainThreadAsyncChannels
{
    fn reset(&mut self)
    {
        *self = Self::default()
    }    
}
