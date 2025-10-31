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
use opencv::core::{RotateFlags, Size_};
struct FrameSizes
{
    frame_size: Size_<i32>,
    rescaled_frame_size: Size_<i32>,
    rotated_rescaled_frame_size: Size_<i32>,
    preview_frame_size: Size_<i32>,
    gui_scale: f32,
    re_scale: f32,
    rotation: Option<RotateFlags>
}
impl FrameSizes
{
    fn new(frame_size: Size_<i32>, rotation: Option<RotateFlags>, gui_scale: f32, re_scale: f32) -> Self
    {
        let mut new_sizes = FrameSizes 
        { 
            frame_size,
            rescaled_frame_size: frame_size, 
            rotated_rescaled_frame_size: frame_size, 
            preview_frame_size: frame_size, 
            gui_scale,
            re_scale,
            rotation
        };
        new_sizes.resize_frame(re_scale);
        new_sizes
    }
    fn resize_frame(&mut self, new_rescale: f32)
    {
        self.re_scale            = new_rescale;
        self.rescaled_frame_size = self.frame_size.get_resized(self.re_scale);
        self.rotate(self.rotation);
    }   
    fn rotate(&mut self, flip: Option<RotateFlags>)
    {
        self.rotation                    = flip;
        self.rotated_rescaled_frame_size = self.rescaled_frame_size.get_rotated(flip);
        self.preview_frame_size          = self.rotated_rescaled_frame_size.get_resized(self.gui_scale);
    }     
    fn resize_gui(&mut self, new_gui_scale: f32)
    {
        self.gui_scale          = new_gui_scale;
        self.preview_frame_size = self.rotated_rescaled_frame_size.get_resized(new_gui_scale);
    }    
    fn get_preview(&self) -> Size_<i32>
    {
        self.preview_frame_size
    }
    fn get_edit(&self) -> Size_<i32>
    {
        self.rotated_rescaled_frame_size
    }
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
        let frame_size: opencv::core::Size_<i32>  = video_info.frame_size.clone();

        let mut frame_sizes = FrameSizes::new(frame_size, options.flip, options.gui_scale, options.re_scale.unwrap_or(1_f32));

        // let rescaled_frame_size = match options.re_scale
        // {
        //     None        => frame_size.clone(),
        //     Some(scale) => frame_size.get_resized(scale)
        // };
        // // frame_size.get_resized(options.re_scale);
        // // rescaled_frame_size.resize(options.re_scale); 

        // let mut rotated_rescaled_frame_size = rescaled_frame_size.get_rotated(options.flip);
        // let mut gui_scale                   = options.gui_scale;
        // // let mut rescaled_frame_size = match  ;
        
        // // rescaled_frame_size.rotate(options.flip).resize(options.re_scale); 

        // let mut preview_frame_size = rotated_rescaled_frame_size.get_resized(gui_scale);

        let file_name    = get_video_name(&options.edit_file_name, "Video Capture Edit");
        let mut path_str = options.edit_file_name.to_str().expect("Not empty");    

        // let mut rotation = options.flip;
        if path_str.ends_with("\"") && path_str.starts_with("\"")
        {
            path_str = path_str.trim_matches('\"');
        }
 
        println!("[Worker] Process file name: {}", file_name);
        println!("[Worker] path_str : {}", path_str);
        

        highgui::named_window(file_name.as_str(), highgui::WINDOW_AUTOSIZE)?;
        // highgui::resize_window_size(file_name.as_str(), preview_frame_size)?;

        println!(">> App (WorkerThread): Let's start processing the video!");


        let mut video_writer: Option<VideoWriter> = if options.should_process
        {
            let fourcc    = VideoWriter::fourcc('m', 'p', '4', 'v')?;
            // let my_writer = VideoWriter::new(path_str, fourcc, video_info.fps, rotated_rescaled_frame_size.clone(), true).expect("Failed init writer!");
            let my_writer = VideoWriter::new(path_str, fourcc, video_info.fps, frame_sizes.get_edit(), true).expect("Failed init writer!");
            // if my_writer.is_opened()?
            // {
            //     worker_channels.send_open_status(true);
            //     println!("[Worker] VideoWriter for \"{path_str}\" sucessfully opened");
            // }
            // else
            // {
            //     worker_channels.send_open_status(false);
            //     println!("Error opening VideoWriter for \"{path_str}\"");
            //     return Ok(capture);
            // }
            Some(my_writer)
        }
        else
        {
            None
        };
        let mut counter     = 0_u32;
        let mut progression = 0_f32;
        // let mut preview_frame_size = rescaled_frame_size.clone();
        while worker_channels.is_not_aborted()
        {
            let mut frame      = Mat::default();
            let mut temp_frame = Mat::default();
            progression = counter as f32 / frame_count as f32;
            worker_channels.send_progression(progression);


            if !capture.read(&mut frame)?
            {
                println!(">> App (WorkerThread): No more frames, back to main!");
                break;
            }
            if let Some(new_gui_scale) = worker_channels.get_last_size_update()
            {
                frame_sizes.resize_gui(new_gui_scale);
                // gui_scale          = new_gui_scale;
                // preview_frame_size = rotated_rescaled_frame_size.get_resized(gui_scale);
                // highgui::resize_window_size(file_name.as_str(), preview_frame_size)?;
            }            
            if let Some(new_flip) = worker_channels.get_updated_flip()
            {
                frame_sizes.rotate(new_flip);
                // rotation = new_flip;
                // // preview_frame_size.rotate(new_flip);
                // rotated_rescaled_frame_size = rescaled_frame_size.get_rotated(rotation);
                // preview_frame_size          = rotated_rescaled_frame_size.get_resized(gui_scale);

                // highgui::resize_window_size(file_name.as_str(), preview_frame_size)?;
            }            
            


            let mut result        = &frame;
            if let Some(rotation) = frame_sizes.rotation//rotation
            {
                rotate(&frame, &mut temp_frame, rotation.code())?;  
                result = &temp_frame;
            }

            if options.preview
            {
                let mut preview = Mat::default();
                opencv::imgproc::resize(&result, &mut preview, frame_sizes.get_preview(), 0.,0., opencv::imgproc::INTER_LINEAR)?;
                highgui::resize_window_size(file_name.as_str(), frame_sizes.get_preview())?;

                // highgui::resize_window_size(file_name.as_str(), preview_frame_size)?;
                highgui::imshow(file_name.as_str(), &preview)?; 
            }
            if let Some(writer) = &mut video_writer
            {
                writer.write(&result)?;
            }

            if highgui::wait_key(10)? > 0 || window_is_closed(file_name.as_str())
            {
                break;
            }

            counter += 1;
        }

        if !window_is_closed(file_name.as_str())
        {
            highgui::destroy_window(file_name.as_str())?;
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