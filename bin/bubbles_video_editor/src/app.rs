use std::ffi::OsStr;
use std::iter::zip;
 
use video_processor::VideoInfo;
use video_processor::VideoProcessor;
use video_processor::ProcessOptions;
use video_processor::RotateFlags;

const RESET_PROGRESS: f32       = 0.0_f32;
const NO_SCALE_CHANGE: f32      = 1.0_f32;
const QUARTER_SCALE_CHANGE: f32 = 0.25_f32;
const DOUBLE_SCALE_CHANGE: f32  = 2.0_f32;
const HALF_SCALE_CHANGE: f32    = 0.5_f32;
const VID_INFO_NAMES: [&'static str; 5] = ["• File name: ", "• Size: ", "• FourCC: ", "• FPS: ", "• Duration: "];
const PLACE_HOLDER_FILELNAME: &str = "";


#[derive(Default, Debug)]
struct VidInfoGui
{
    has_some_info: bool,
    vid_info_result: [String; 5],
}
impl VidInfoGui
{
    fn try_update(&mut self, file_path: &std::path::PathBuf, infos: &Option<VideoInfo>)
    {
        self.has_some_info = infos.is_some();

        if let Some(infos) = infos
        {
            self.vid_info_result[0] = format!("{}", video_processor::get_video_name(file_path, "Video Capture"));//infos.file_name);
            self.vid_info_result[1] = format!("{}x{}", infos.frame_size.width, infos.frame_size.height);
            self.vid_info_result[2] = format!("{}{}{}{}", infos.fourcc_codec.0, infos.fourcc_codec.1, infos.fourcc_codec.2, infos.fourcc_codec.3);
            self.vid_info_result[3] = format!("{:.1}", infos.fps );
            let total_secs   = (infos.frame_count as f64 / infos.fps) as u64;
            let min          = total_secs / 60_u64;
            let reminder_sec = total_secs % 60_u64;
            self.vid_info_result[4] = format!("{:.1}min {:.1}s", min, reminder_sec);
        };
    }    
    fn show_rows(&self, ui: &mut egui::Ui)
    {
        for (info_name, info_result) in zip(VID_INFO_NAMES, &self.vid_info_result)
        {
            ui.label(info_name);
            if self.has_some_info
            {
                ui.label(info_result);
            }
            else 
            {
                ui.label("");
            }
            ui.end_row();
        }
    }
}

 
 
#[derive(PartialEq)]
enum RotationRadio
{
    First(Option<RotateFlags>),
    Second(Option<RotateFlags>),
    Third(Option<RotateFlags>),
    Forth(Option<RotateFlags>),
}
#[derive(PartialEq)]

enum ProcessModes
{
    PreviewOnly,
    PreviewAndProcess    
}

impl RotationRadio 
{
    fn get(&self) -> Option<RotateFlags>
    {
        match self 
        {
            Self::First(value)  => *value,
            Self::Second(value) => *value,
            Self::Third(value)  => *value,
            Self::Forth(value)  => *value,
        }
    }    
}

fn create_default_edit_path(file_name: &std::path::PathBuf, placer_holder: &str) -> std::path::PathBuf
{
    let default_directory = match std::env::current_dir()
    {
        Ok(cwd) => cwd,
        Err(_)  => std::path::PathBuf::from(""),
    };

    let parent    = file_name.parent().unwrap_or(&default_directory);
    let extension = file_name.extension().unwrap_or(OsStr::new("no_extension"));
    let mut new_file_stem = file_name.file_stem().unwrap_or(OsStr::new("empty_file_name")).to_owned();

    println!("DEBUG: Filename {}", new_file_stem.to_str().expect("String not empty"));
    println!("DEBUG: extension {}", extension.to_str().expect("String not empty"));
    new_file_stem.push(placer_holder);
    
    let mut processed_file_path = parent.join(new_file_stem);
    processed_file_path.set_extension(extension);

    processed_file_path
}

enum VideoMode
{
    Play(&'static str),
    Pause(&'static str),
}
impl VideoMode
{
    const PLAY: VideoMode = VideoMode::Play("Play");
    const PAUSE: VideoMode = VideoMode::Pause("Pause");
    fn get_name(&self) -> &'static str 
    {
        match self
        {
            VideoMode::Pause(s) => s,
            VideoMode::Play(s) => s,
        }
    }    
}



pub struct BubblesVideoEditor 
{
    dropped_files: Vec<egui::DroppedFile>,
    label: String,
    edit_file_buffer: String,
    has_new_edit_file_name: bool,
    progress: f32,
    app: VideoProcessor,
    file_name: Option<std::path::PathBuf>,
    has_tried_opening: bool, 
    flip_choice: RotationRadio,
    process_mode: ProcessModes,
    gui_scale: f32,
    new_image_scale: f32,
    edit_file_name: std::path::PathBuf,
    video_info_gui: VidInfoGui,
    next_video_mode: VideoMode,
}
 
impl Default for BubblesVideoEditor 
{
    fn default() -> Self 
    {
        Self {
            dropped_files: Vec::<egui::DroppedFile>::default(),
            edit_file_buffer: String::default(),
            label: PLACE_HOLDER_FILELNAME.to_owned(),
            has_new_edit_file_name: false,
            progress: RESET_PROGRESS,
            app: VideoProcessor::default(),
            file_name: None, 
            has_tried_opening: false,
            flip_choice: RotationRadio::First(None),
            process_mode: ProcessModes::PreviewOnly,
            gui_scale: QUARTER_SCALE_CHANGE,
            new_image_scale: NO_SCALE_CHANGE,
            edit_file_name: std::path::PathBuf::default(), // could be a "new pathbuff...",
            video_info_gui: VidInfoGui::default(),
            next_video_mode: VideoMode::PAUSE,
        }
    }
}
 


impl BubblesVideoEditor 
{
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self 
    {
        cc.egui_ctx.set_theme(egui::Theme::Dark);
        Default::default()
    }

    fn show_menu(ctx: &egui::Context) 
    {
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| 
        {
            egui::MenuBar::new().ui(ui, |ui| 
            {
                ui.menu_button("File", |ui| 
                {
                    if ui.button("Quit").clicked() 
                    {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });
                ui.add_space(16.0);
                // egui::widgets::global_theme_preference_buttons(ui);
            });
        });
    }       


    fn handle_file_opening(&mut self, ui: &mut egui::Ui)
    {
        ui.add_enabled(!self.app.has_launched_process(), 
            egui::TextEdit::singleline(&mut self.label));

        // Drag n drop ...
        if !self.dropped_files.is_empty() 
        {
            let mut file      = self.dropped_files.pop().unwrap(); 
            if let Some(path) = file.path.take()
            {
                self.file_name = Some(path)
            }   
        }
        // or file dialog!
        if ui.add_enabled(!self.app.has_launched_process(), egui::Button::new("Open file…")).clicked() 
        {
            if let Some(path) = rfd::FileDialog::new().pick_file() 
            {
                self.file_name = Some(path);
            }
        }

    
        if let Some(file_path) = self.file_name.take()
        {
            if let Err(opencv_err) = self.app.unload_video() // release video if any
            {
                println!("Error Releasing video: {opencv_err}");
            };
    
            self.label = file_path.display().to_string();
    
            self.app.try_grab_video(&file_path);
            self.video_info_gui.try_update(&file_path, &self.app.video_info);
    
            if self.app.has_video() //is_video_loaded()
            {
                self.edit_file_name   = create_default_edit_path(&file_path, "_edit");
                self.edit_file_buffer = String::from(self.edit_file_name.to_str().expect("edit_file_buffer: Could not Path to &str."));
            }
            self.has_tried_opening = true;
        }
    
        if self.app.has_video()  
        {
            ui.label("Video loaded successfully!");
            self.has_tried_opening = false;
        }
        else if self.has_tried_opening 
        {
            ui.label("Cannot open video file!");
        }
        else  
        {
            ui.label("No files opened");
        }
    }

    fn handle_video_edit_choice(&mut self, ui: &mut egui::Ui)
    {
        ui.label("Rotate video:");
     
  
        ui.horizontal(|ui|
        {
            ui.radio_value(&mut self.flip_choice, RotationRadio::First(None), "No Rotation");
            ui.radio_value(&mut self.flip_choice, RotationRadio::Second(Some(RotateFlags::ROTATE_180)), "Rotate 180");
            ui.radio_value(&mut self.flip_choice, RotationRadio::Third(Some(RotateFlags::ROTATE_90_CLOCKWISE)), "Rotate 90 Clockwise");
            ui.radio_value(&mut self.flip_choice, RotationRadio::Forth(Some(RotateFlags::ROTATE_90_COUNTERCLOCKWISE)), "Rotate 90 Counter Clockwise");
        });
        // ui.add_enabled_ui(!self.app.has_launched_process() && self.app.has_video(), |ui|// .is_video_loaded(), |ui|
        // {
        ui.label("Scale changer");
        ui.add(egui::Slider::new(&mut self.new_image_scale, 0.1..=2.0));
        ui.horizontal(|ui|
        {
            ui.label("Scale presets:");
            if ui.button("0.25").clicked()
            {
                self.new_image_scale = QUARTER_SCALE_CHANGE;
            }
            if ui.button("0.5").clicked()
            {
                self.new_image_scale = HALF_SCALE_CHANGE;
            }
            if ui.button("1.0").clicked()
            {
                self.new_image_scale = NO_SCALE_CHANGE;
            }
            if ui.button("2.0").clicked()
            {
                self.new_image_scale = DOUBLE_SCALE_CHANGE;
            }
        });
        ui.horizontal(|ui|
        {
            ui.label("Output path:");
            if ui.text_edit_singleline(&mut self.edit_file_buffer).changed()
            {
                self.has_new_edit_file_name = true;
            }
            if ui.button("Set").clicked()
            {
                self.edit_file_name         = std::path::PathBuf::from(&self.edit_file_buffer);
                self.has_new_edit_file_name = false;
            }
            if self.has_new_edit_file_name
            {
                ui.label("path not set!")    
            }
            else {
                ui.label("Set!")
            }
        });
        // });

        // This will dispatch new values to the processing thread, if process is launched!
        if self.process_mode == ProcessModes::PreviewOnly
        {
            if let Err(e) = self.app.set_flip(self.flip_choice.get())
            {
                println!("Error: {e}");
            }
            if let Err(e) = self.app.set_rescale(self.new_image_scale)
            {
                println!("Error: {e}");
            }
        }
    }
        
    fn show_video_info(&mut self, ui: &mut egui::Ui)
    {
        egui::Grid::new("vid_info")
        .num_columns(2)
        .show(ui, |ui|
        {
            self.video_info_gui.show_rows(ui);
        });
    }

    fn handle_video_processing(&mut self, ui: &mut egui::Ui) 
    {
        egui::Grid::new("process_mode")
            .num_columns(2)
            .show(ui, |ui|
            {
                ui.label("Current mode ");
                match self.process_mode
                {
                    ProcessModes::PreviewOnly =>        ui.heading("Preview"),
                    ProcessModes::PreviewAndProcess =>  ui.heading("Output file"),
                };
                ui.end_row();
            });
    
    
        ui.horizontal(|ui|
        {
            if ui.add_enabled(self.app.has_video() && !self.app.has_launched_process(), egui::Button::new("Launch")).clicked()
            {
                let edit_file_name = self.edit_file_name.clone();
                let re_scale       = match self.new_image_scale
                {
                    NO_SCALE_CHANGE => None,
                    _               => Some(self.new_image_scale)
                };
                let flip           = self.flip_choice.get();
                let should_process = self.process_mode == ProcessModes::PreviewAndProcess;
                let preview        = true;
                let gui_scale      = self.gui_scale;
                let options = ProcessOptions
                {
                    gui_scale,
                    edit_file_name,
                    flip,
                    should_process,
                    preview,
                    re_scale,
                };
                self.progress = RESET_PROGRESS;
                self.app.dispatch_video_process(options);
            }
            ui.add_enabled_ui(self.app.has_launched_process(), |ui|
            {
                if ui.button(self.next_video_mode.get_name()).clicked()
                {   
                    match self.next_video_mode
                    {
                        VideoMode::Pause(_) => 
                        {
                            if let Err(e) = self.app.pause_video()
                            {
                                println!("{e}");
                            }
                            self.next_video_mode = VideoMode::PLAY;
                        }    
                        VideoMode::Play(_) => 
                        {
                            if let Err(e) = self.app.resume_video()
                            {
                                println!("{e}");
                            }
                            self.next_video_mode = VideoMode::PAUSE;
                        }    
                    }
                }
            });


            if ui.add_enabled(self.app.has_launched_process(), egui::Button::new("Abort")).clicked()
            {
                if self.app.try_abort()
                {
                    println!("GUI: Message sent!"); 
                }
                else
                {
                    println!("Failure sending message!"); 
                }
            }
            if ui.add_enabled(self.app.has_video() && !self.app.has_launched_process(), egui::RadioButton::new(self.process_mode == ProcessModes::PreviewOnly, "Preview")).clicked()
            {
                self.process_mode = ProcessModes::PreviewOnly;
            }
            if ui.add_enabled(self.app.has_video() && !self.app.has_launched_process(), egui::RadioButton::new(self.process_mode == ProcessModes::PreviewAndProcess, "Output file")).clicked()
            {
                self.process_mode = ProcessModes::PreviewAndProcess;
            }
            if self.app.has_launched_process()
            {
                if let Some(progression) = self.app.get_progression()
                {
                    self.progress = progression;
                    println!("progress: {}", self.progress);
                };
                let progress_bar = egui::ProgressBar::new(self.progress)
                    .show_percentage()
                    .animate(true);
                ui.add(progress_bar);
            }

            
        });
    
        if self.app.has_launched_process()
        {
            ui.horizontal(|ui|
            {
                ui.label("Preview Window Size:");
                ui.add(egui::Slider::new(&mut self.gui_scale, 0.1..=2.0));
    
                ui.label("Presets:");
                if ui.button("0.25").clicked()
                {
                    self.gui_scale = QUARTER_SCALE_CHANGE;
                }
                if ui.button("0.5").clicked()
                {
                    self.gui_scale = HALF_SCALE_CHANGE;
                }
                if ui.button("1.0").clicked()
                {
                    self.gui_scale = NO_SCALE_CHANGE;
                }
                if ui.button("2.0").clicked()
                {
                    self.gui_scale = DOUBLE_SCALE_CHANGE;
                }
            });
        }
        if let Err(e) = self.app.set_gui_scale(self.gui_scale)
        {
            println!("Error: {e}");
        }
        if self.app.has_launched_process() && self.app.is_process_finished()
        {
            match self.app.handle_thread_join()
            {
                Ok(progress) => println!("Thread joined successfully final progress: {}%.", progress*100_f32),
                Err(e)       =>
                { 
                    println!("OpenCV Error in second thread: {e}")
                },
            } 
        }
    }
}

impl eframe::App for BubblesVideoEditor 
{
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) 
    {
        BubblesVideoEditor::show_menu(ctx);

        egui::CentralPanel::default().show(ctx, |ui| 
        {
            ui.vertical(|ui| 
            {
                ui.heading("Video file: ");
                ui.label("Drag & drop into the app or use the file dialog!");
            });
            ui.horizontal(|ui| 
            {
                self.handle_file_opening(ui);
            });

            ui.separator();
            
            ui.heading("Video infos:");
            self.show_video_info(ui);

            
            //// Video editor ///
            ui.separator();
            ui.heading("Video Editor:");
            self.handle_video_edit_choice(ui);


            //// Video Processor ///
            ui.separator();
            ui.heading("Video Processor");
            self.handle_video_processing(ui);

            if !self.app.has_launched_process()
            {
                preview_files_being_dropped(ctx);
                //// Collect dropped files ////
                ctx.input(|i| 
                {
                    if !i.raw.dropped_files.is_empty() 
                    {
                        self.dropped_files.clone_from(&i.raw.dropped_files);
                    }
                });
            }

            //// bottom ////
            ui.horizontal(|ui|
            {
                ui.spacing_mut().item_spacing.x = 0.0;
                ui.label("Powered by ");
                ui.hyperlink_to("egui", "https://github.com/emilk/egui");
            });
            ui.hyperlink_to("Source code.","https://github.com/so-groenen/bubbles_video_editor");
        });
    }
}




/// See: https://github.com/emilk/egui/tree/main/examples/file_dialog !
fn preview_files_being_dropped(ctx: &egui::Context) 
{
    use egui::{Align2, Color32, Id, LayerId, Order, TextStyle};
    use std::fmt::Write as _;

    if !ctx.input(|i| i.raw.hovered_files.is_empty()) 
    {
        let text = ctx.input(|i| 
        {
            let mut text = "Dropping file:\n".to_owned();
            for file in &i.raw.hovered_files 
            {
                if let Some(path) = &file.path
                {
                    write!(text, "\n{}", path.file_name().unwrap().display()).ok();
                }
                else if !file.mime.is_empty() 
                {
                    write!(text, "\n{}", file.mime).ok();
                } 
                else 
                {
                    text += "\n???";
                }
            }
            text
        });

        let painter = ctx.layer_painter(LayerId::new(Order::Foreground, Id::new("file_drop_target")));

        let screen_rect = ctx.screen_rect();
        painter.rect_filled(screen_rect, 0.0, Color32::from_black_alpha(192));
        painter.text(
            screen_rect.center(),
            Align2::CENTER_CENTER,
            text,
            TextStyle::Heading.resolve(&ctx.style()),
            Color32::WHITE,
        );
    }
}