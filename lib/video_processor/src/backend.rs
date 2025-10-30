pub mod helper_function;
use helper_function::*;
pub use helper_function::get_video_name;
use opencv::videoio::VideoWriter;
// use opencv::videoio::VideoCapture;

mod data_structures;
pub use crate::backend::data_structures::*;

use opencv::prelude::*;
use opencv::{videoio::{self}, highgui, core::{rotate}};
use std::thread::{self};


pub fn process_video_thread(mut capture: videoio::VideoCapture, 
                    options: ProcessOptions,
                    thread_pool: &mut MyThreadPool2,
                    mut worker_channels: WorkerThreadAsyncChannels,
                    )
{
    let handle = thread::spawn(move ||
    {
        let video_info = VideoInfo::new(&capture)?;
        let frame_count = video_info.frame_count;
        let mut result_frame_size = video_info.frame_size.clone();

        result_frame_size.rotate(options.flip).resize(options.re_scale); 


        let file_name  = get_video_name(&options.edit_file_name, "Video Capture Edit");
        let mut path_str = options.edit_file_name.to_str().expect("Not empty");    


        if path_str.ends_with("\"") && path_str.starts_with("\"")
        {
            path_str = path_str.trim_matches('\"');
        }
 
        println!("[Worker] Process file name: {}", file_name);
        println!("[Worker] path_str : {}", path_str);
        
        
        highgui::named_window(file_name.as_str(), highgui::WINDOW_NORMAL)?;
        highgui::resize_window_size(file_name.as_str(), result_frame_size.get_resized(options.gui_scale))?;

        println!(">> App (WorkerThread): Let's start processing the video!");


        let mut video_writer = if options.should_process
        {
            let fourcc = VideoWriter::fourcc('m', 'p', '4', 'v')?;
            let my_writer = 
            VideoWriter::new(path_str, fourcc, video_info.fps, result_frame_size.clone(), true).expect("Failed init writer!");
            if my_writer.is_opened()?
            {
                worker_channels.send_open_status(true);
                println!("[Worker] VideoWriter for \"{path_str}\" sucessfully opened");
            }
            else
            {
                worker_channels.send_open_status(false);
                println!("Error opening VideoWriter for \"{path_str}\"");
                return Ok(capture);
            }
            Some(my_writer)
        }
        else
        {
            None
        };
        let mut counter:     u32 = 0;
        let mut progression: f32 = 0f32;
        while worker_channels.is_not_aborted()
        {
            let mut frame = Mat::default();
            let mut new_frame = Mat::default();

            progression = counter as f32 / frame_count as f32;
            worker_channels.send_progression(progression);

            if let Some(new_gui_scale) = worker_channels.get_last_size_update()
            {
                highgui::resize_window_size(file_name.as_str(), result_frame_size.get_resized(new_gui_scale))?;
            }

            if !capture.read(&mut frame)?
            {
                println!(">> App (WorkerThread): No more frames, back to main!");
                break;
            }
            if let Some(scaling) = options.re_scale
            {
                let dsize = video_info.frame_size.get_resized(scaling);
                opencv::imgproc::resize(&frame, &mut new_frame, dsize, 0.,0., opencv::imgproc::INTER_LINEAR)?;
                frame.clone_from(&new_frame);
            }

            let mut result = &frame;
            if let Some(rotation) = options.flip
            {
                rotate(&frame, &mut new_frame, rotation.code())?;  
                result = &new_frame;
            }
            if options.preview
            {
                highgui::imshow(file_name.as_str(), result)?; 
            }
            if let Some(writer) = &mut video_writer
            {
                writer.write(&result)?;
            }
            if highgui::wait_key(10)? > 0 
            {
                break;
            }
            counter += 1;
        }

        if let Some(mut writer) = video_writer.take()
        {
            println!("VideoWriter closed at {}%",progression*100f32);
            writer.release()?;
        }

        highgui::destroy_window(file_name.as_str())?;//.unwrap();
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