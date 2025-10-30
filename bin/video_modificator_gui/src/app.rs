use std::ffi::OsStr;
use std::iter::zip;

use video_processor::VideoInfo;
use video_processor::VideoProcessor;
use video_processor::ProcessOptions;
use video_processor::RotateFlags;

const RESET_PROGRESS: f32       = 0f32;
const NO_SCALE_CHANGE: f32      = 1f32;
const QUARTER_SCALE_CHANGE: f32 = 0.25f32;
const DOUBLE_SCALE_CHANGE: f32  = 2f32;
const HALF_SCALE_CHANGE: f32    = 0.5f32;
const VID_INFO_NAMES: [&'static str; 5] = ["• File name: ", "• Size: ", "• FourCC: ", "• FPS: ", "• Frame_counter "];

 
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
        match infos
        {
            Some(infos) => 
            {
                self.has_some_info = true;
                self.vid_info_result[0] = format!("{}", video_processor::get_video_name(file_path, "Video Capture"));//infos.file_name);
                self.vid_info_result[1] = format!("{}x{}", infos.frame_size.width, infos.frame_size.height);
                self.vid_info_result[2] = format!("{}{}{}{}", infos.fourcc_codec.0, infos.fourcc_codec.1, infos.fourcc_codec.2, infos.fourcc_codec.3);
                self.vid_info_result[3] = format!("{:.1}", infos.fps );
                self.vid_info_result[4] = format!("{}", infos.frame_count);
            }
            None => self.has_some_info = false
        };
    }    
    fn show_rows(&self, ui: &mut egui::Ui)
    {
        // for i in 0..self.vid_info_result.len()
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
            Self::First(value )  => *value,
            Self::Second(value ) => *value,
            Self::Third(value )  => *value,
            Self::Forth(value )  => *value,
        }
    }    
}

fn create_default_edit_path(file_name: &std::path::PathBuf, placer_holder: &str) -> std::path::PathBuf
{
    let default_directory = match std::env::current_dir()
    {
        Ok(cwd) => cwd,
        Err(_) => std::path::PathBuf::from(""),
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

pub struct VideoModificator 
{
    label: String,
    edit_file_buffer: String,
    has_new_edit_file_name: bool,
    progress: f32,
    app: VideoProcessor,
    file_name: std::path::PathBuf,
    has_tried_opening: bool, 
    flip_choice: RotationRadio,
    process_mode: ProcessModes,
    gui_scale: f32,
    new_image_scale: f32,
    edit_file_name: std::path::PathBuf,
    video_info_gui: VidInfoGui,
}
 
impl Default for VideoModificator 
{
    fn default() -> Self 
    {
        Self {
            edit_file_buffer: "".to_owned(),
            label: "my_file.mp4".to_owned(),
            has_new_edit_file_name: false,
            progress: RESET_PROGRESS,
            app: VideoProcessor::default(),
            file_name: std::path::PathBuf::default(),
            has_tried_opening: false,
            // video_info: None,
            flip_choice: RotationRadio::First(None),
            process_mode: ProcessModes::PreviewOnly,
            gui_scale: QUARTER_SCALE_CHANGE,
            new_image_scale: NO_SCALE_CHANGE,
            edit_file_name: std::path::PathBuf::default(), // could be a "new pathbuff...",
            video_info_gui: VidInfoGui::default(),
        }
    }
}

impl VideoModificator 
{
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self 
    {
        cc.egui_ctx.set_theme(egui::Theme::Dark);
        Default::default()
    }
}

impl eframe::App for VideoModificator 
{
    // /// Called by the framework to save state before shutdown.
    // fn save(&mut self, storage: &mut dyn eframe::Storage) {
    //     eframe::set_value(storage, eframe::APP_KEY, self);
    // }
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) 
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

        egui::CentralPanel::default().show(ctx, |ui| 
        {
            ui.horizontal(|ui| 
            {
                ui.heading("Video file: ");
                ui.text_edit_singleline(&mut self.label);
 
            });

            ui.horizontal(|ui| 
            {
                if ui.button("Open").clicked()
                {
                    if let Err(opencv_err) = self.app.unload_video()
                    {
                        println!("Error Releasing video: {opencv_err}");
                    };

                    self.has_tried_opening = true;
                    if self.label.ends_with("\"") && self.label.starts_with("\"")
                    {
                        let trimmed_input = self.label.trim_matches('\"');
                        self.file_name.set_file_name(&trimmed_input);
                    }
                    else {
                        self.file_name.set_file_name(&self.label);
                    }

                    self.app.try_grab_video(&self.file_name);
                    self.video_info_gui.try_update(&self.file_name,&self.app.video_info);

                    if self.app.is_video_loaded()
                    {
                        self.edit_file_name   = create_default_edit_path(&self.file_name, "_edit");
                        self.edit_file_buffer = String::from(self.edit_file_name.to_str().expect("edit_file_buffer: Could not Path to &str."));
                        // ui.label("Video loaded successfully!");
                    }
                }

                if self.app.is_video_loaded()
                {
                    ui.label("Video loaded successfully!");
                }
                else if self.has_tried_opening 
                {
                    ui.label("No file found!");
                }
                else  
                {
                    ui.label("No files opened");
                }
            });

            ui.separator();
            
            ui.heading("Video infos:");
            egui::Grid::new("vid_info")
            .num_columns(2)
            .show(ui, |ui|
            {
                self.video_info_gui.show_rows(ui);
            });

            ui.separator();
            ui.heading("Video Editor:");
            ui.label("Rotate video:");
            ui.horizontal(|ui|
            {
                ui.radio_value(&mut self.flip_choice, RotationRadio::First(None), "No Rotation");
                ui.radio_value(&mut self.flip_choice, RotationRadio::Second(Some(RotateFlags::ROTATE_180)), "Rotate 180");
                ui.radio_value(&mut self.flip_choice, RotationRadio::Third(Some(RotateFlags::ROTATE_90_CLOCKWISE)), "Rotate 90 Clockwise");
                ui.radio_value(&mut self.flip_choice, RotationRadio::Forth(Some(RotateFlags::ROTATE_90_COUNTERCLOCKWISE)), "Rotate 90 Counter Clockwise");
            });
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
            


            if self.app.is_video_loaded()
            {
                ui.horizontal(|ui|
                {
                    ui.label("Output path:");
                    if ui.text_edit_singleline(&mut self.edit_file_buffer).changed()
                    {
                        self.has_new_edit_file_name = true;
                    }
                    if ui.button("Set").clicked()
                    {
                        // does not need to be option
                        self.edit_file_name = std::path::PathBuf::from(&self.edit_file_buffer);
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
            }

            ui.separator();
            ui.heading("Video Processor");
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
                    let re_scale = match self.new_image_scale
                    {
                        NO_SCALE_CHANGE => None,
                        _ =>               Some(self.new_image_scale)
                    };
                    let flip = self.flip_choice.get();
                    let should_process = self.process_mode == ProcessModes::PreviewAndProcess;
                    let preview= true;
                    let gui_scale = self.gui_scale;
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
                    // let have_transmitter  = self.app.try_abort().expect("Failed sending abort message even though we have a transmitter");
                    // if have_transmitter 
                    // {
                    //     println!("GUI: Message sent!"); 
                    // }
                    // else
                    // {
                    //     println!(">> App (Main): Worker already returned, reciever destroyed"); 
                    // }
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

            // if self.app.has_launched_process()
            // {
            ui.horizontal(|ui|
            {
                ui.label("Gui scale:");
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

            // }
            // if let Err(e) = self.app.set_gui_scale(self.gui_scale)
            // {
            //     println!("Error: {e}");
            // }
            self.app.set_gui_scale(self.gui_scale);
            if self.app.has_launched_process() && self.app.is_process_finished()
            {
                match self.app.handle_thread_join()
                {
                    Ok(progress) => println!("Thread joined successfully final progress: {}%.", progress*100f32),
                    Err(e) =>
                    { 
                        // ui.label(format!("OpenCV Error while processing: {e}"));
                        println!("OpenCV Error in second thread: {e}")
                    },
                } 
            }

            ui.add(egui::github_link_file!(
                "https://github.com/emilk/eframe_template/blob/main/",
                "Source code."
            ));

            ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                powered_by_egui_and_eframe(ui);
                egui::warn_if_debug_build(ui);
            });
        });
    }
}

fn powered_by_egui_and_eframe(ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        ui.label("Powered by ");
        ui.hyperlink_to("egui", "https://github.com/emilk/egui");
        ui.label(" and ");
        ui.hyperlink_to(
            "eframe",
            "https://github.com/emilk/egui/tree/master/crates/eframe",
        );
        ui.label(".");
    });
}
