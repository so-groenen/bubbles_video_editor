pub mod helper_function;
use helper_function::*;
pub use helper_function::get_video_name;
use opencv::videoio::VideoWriter;

mod data_structures;
pub use crate::backend::data_structures::*;

use opencv::prelude::*;
use opencv::{videoio::{self}, highgui, core::{rotate}};
use std::thread::{self};


// Trick from https://stackoverflow.com/a/9321629
fn window_is_closed(winname: &str) -> bool
{
    highgui::get_window_property(winname, highgui::WindowPropertyFlags::WND_PROP_FULLSCREEN.into()).is_err()
}


pub fn process_video_thread(mut capture: videoio::VideoCapture, 
                            options: ProcessOptions,
                            thread_pool: &mut MyThreadPool2,
                            mut worker_channels: WorkerThreadAsyncChannels,
                            )
{
    let handle = thread::spawn(move ||
    {
        let video_info  = VideoInfo::new(&capture)?;
        let frame_count = video_info.frame_count;
 
        let mut frame_sizes   = FrameSizeManager::new(video_info.frame_size, options.flip, options.gui_scale, options.re_scale.unwrap_or(1_f32));
        let default_file_name = "Video Capture Edit";
        let win_name          = &options.get_video_name(default_file_name)[..];
        let path_str          = options.get_edit_path_str();

        highgui::named_window(win_name, highgui::WINDOW_AUTOSIZE)?;

 
        println!("[Worker] Process file name: {}", win_name);
        println!("[Worker] path_str : {}", path_str);
        println!(">> App (WorkerThread): Let's start processing the video!");


        let mut video_writer: Option<VideoWriter> = None;
        if options.should_process
        {
            let fourcc   = VideoWriter::fourcc('m', 'p', '4', 'v')?;
            let writer   = VideoWriter::new(path_str, fourcc, video_info.fps, frame_sizes.get_edit(), true).expect("Failed init writer!");
            video_writer = Some(writer)
        };
        
        let mut counter     = 0_u32;
        let mut progression = 0_f32;
        while worker_channels.is_not_aborted()
        {
            let mut frame      = Mat::default();
            let mut temp_frame = Mat::default();

            progression = counter as f32 / frame_count as f32;
            worker_channels.send_progression(progression);
            
            if !capture.read(&mut frame)? || highgui::wait_key(10)? > 0 || window_is_closed(win_name)
            {
                break;
            }


            
            frame_sizes.update_from_main(&mut worker_channels);


            let mut result        = &frame;
            if let Some(rotation) = frame_sizes.get_rotation()
            {
                rotate(&frame, &mut temp_frame, rotation.code())?;  
                result = &temp_frame;
            }
            if options.preview
            {
                let mut preview = Mat::default();
                opencv::imgproc::resize(&result, &mut preview, frame_sizes.get_preview(), 0.,0., opencv::imgproc::INTER_LINEAR)?;
                highgui::resize_window_size(win_name, frame_sizes.get_preview())?;
                highgui::imshow(win_name, &preview)?; 
            }
            if let Some(writer) = &mut video_writer
            {
                writer.write(&result)?;
            }
            counter += 1;
        }

        if !window_is_closed(win_name)
        {
            highgui::destroy_window(win_name)?;
        }

        if let Some(mut writer) = video_writer.take()
        {
            println!("VideoWriter closed at {}%",progression*100_f32);
            writer.release()?;
        }
        Ok(capture)
    });

    thread_pool.push(handle);
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