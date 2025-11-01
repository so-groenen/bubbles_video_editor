use video_processor::VideoInfo;
use std::iter::zip;

const VID_INFO_NAMES: [&'static str; 5] = ["• File name: ", "• Size: ", "• FourCC: ", "• FPS: ", "• Duration: "];


#[derive(Default, Debug)]
pub struct VidInfoGui
{
    has_some_info: bool,
    vid_info_result: [String; 5],
}
impl VidInfoGui
{
    pub fn try_update(&mut self, file_path: &std::path::PathBuf, infos: &Option<VideoInfo>)
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
    pub fn show_rows(&self, ui: &mut egui::Ui)
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
