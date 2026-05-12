use pyo3::prelude::*;
use std::process::{Command, Stdio};
use std::io::{Write, BufRead, BufReader};
use serde_json::Value;
use rayon::prelude::*;

#[pyclass]
#[derive(Clone)]
pub struct VideoMetadata {
    #[pyo3(get)] pub filename: String,
    #[pyo3(get)] pub width: i64,
    #[pyo3(get)] pub height: i64,
    #[pyo3(get)] pub codec: String,
    #[pyo3(get)] pub duration: f64,
    #[pyo3(get)] pub pix_fmt: String,
    #[pyo3(get)] pub bitrate: String,
}

#[pyclass]
pub struct AnchEngine;

#[pymethods]
impl AnchEngine {
    #[new] fn new() -> Self { AnchEngine }

    fn probe(&self, path: String) -> PyResult<VideoMetadata> {
        let output = Command::new("ffprobe")
            .args(["-v", "quiet", "-print_format", "json", "-show_format", "-show_streams"])
            .arg(&path)
            .output()
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyIOError, _>(e.to_string()))?;

        let v: Value = serde_json::from_slice(&output.stdout)
            .map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid ffprobe output"))?;

        let streams = v["streams"].as_array().ok_or_else(|| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>("No streams found"))?;
        let video_stream = streams.iter().find(|s| s["codec_type"] == "video").ok_or_else(|| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>("No video stream found"))?;
        let format = &v["format"];

        Ok(VideoMetadata {
            filename: format["filename"].as_str().unwrap_or("unknown").to_string(),
            width: video_stream["width"].as_i64().unwrap_or(0),
            height: video_stream["height"].as_i64().unwrap_or(0),
            codec: video_stream["codec_name"].as_str().unwrap_or("unknown").to_string(),
            duration: format["duration"].as_str().unwrap_or("0").parse().unwrap_or(0.0),
            pix_fmt: video_stream["pix_fmt"].as_str().unwrap_or("unknown").to_string(),
            bitrate: format["bit_rate"].as_str().unwrap_or("0").to_string(),
        })
    }

    fn get_blackbar_coords(&self, path: String) -> PyResult<String> {
        let meta = self.probe(path.clone())?;
        let output = Command::new("ffmpeg")
            .args(["-hide_banner", "-loglevel", "info", "-i", &path, "-vf", "cropdetect=24:16:0", "-vframes", "100", "-f", "null", "-"])
            .output()
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyIOError, _>(e.to_string()))?;

        let stderr = String::from_utf8_lossy(&output.stderr);
        let coords = stderr.lines()
            .filter(|l| l.contains("crop="))
            .last()
            .and_then(|l| l.split("crop=").last())
            .map(|s| s.trim().to_string())
            .unwrap_or_else(|| format!("{}:{}:0:0", meta.width, meta.height));
        Ok(coords)
    }

    fn transform(&self, input: String, output: String, codec: String, res: String, crop: String, flags: String) -> PyResult<bool> {
        let mut ffmpeg = Command::new("ffmpeg");
        ffmpeg.arg("-y").arg("-hide_banner").arg("-loglevel").arg("error").arg("-stats");
        ffmpeg.args(["-i", &input]);
        ffmpeg.args(["-map", "0", "-map_metadata", "-1", "-map_chapters", "0"]);

        let clean_crop = crop.replace("crop=", "");
        let filter_chain = format!("crop={},scale=w='trunc(oh*a/16)*16':h={}:flags=lanczos,setsar=1", clean_crop, res);
        ffmpeg.arg("-vf").arg(filter_chain);

        ffmpeg.args(["-vsync", "cfr", "-fps_mode", "cfr"]); 
        ffmpeg.arg("-c:v").arg(&codec);

        if codec.contains("x265") || codec.contains("hevc") {
            ffmpeg.args(["-pix_fmt", "yuv420p10le", "-profile:v", "main10"]);
            if !flags.contains("x265-params") {
                ffmpeg.args(["-x265-params", "aq-mode=3:strong-intra-smoothing=0:sao=0"]);
            }
        } else if codec.contains("x264") {
            ffmpeg.args(["-pix_fmt", "yuv420p", "-profile:v", "high", "-level", "4.1"]);
        }

        ffmpeg.args(["-c:a", "copy", "-c:s", "copy"]);

        for flag in flags.split_whitespace() {
            ffmpeg.arg(flag);
        }
        
        ffmpeg.arg(&output).stderr(Stdio::piped());

        let args: Vec<String> = ffmpeg.get_args().map(|a| a.to_string_lossy().into_owned()).collect();
        println!("\n  > [DEBUG] CMD: ffmpeg {}", args.join(" "));

        let mut child = ffmpeg.spawn().map_err(|e| PyErr::new::<pyo3::exceptions::PyIOError, _>(e.to_string()))?;

        if let Some(stderr) = child.stderr.take() {
            let reader = BufReader::new(stderr);
            for line in reader.lines().filter_map(Result::ok) {
                if let Some(pos) = line.rfind("frame=") {
                    print!("\r\x1b[2K  > [⚓ ANCHORR] ENCODE | {}", &line[pos..].trim());
                    let _ = std::io::stdout().flush();
                }
            }
        }

        let status = child.wait().map_err(|e| PyErr::new::<pyo3::exceptions::PyIOError, _>(e.to_string()))?;
        
        if !status.success() {
            println!("\n  > [⚓ ANCHORR] FFmpeg exited with an error.");
        }
        
        Ok(status.success())
    }

    fn remux(&self, input: String, output: String) -> PyResult<bool> {
        let status = Command::new("ffmpeg")
            .args(["-hide_banner", "-loglevel", "quiet", "-nostats", "-i", &input, "-c", "copy", "-map", "0", "-map_metadata", "-1", "-y"])
            .arg(&output).status().map_err(|e| PyErr::new::<pyo3::exceptions::PyIOError, _>(e.to_string()))?;
        Ok(status.success())
    }

    fn batch(&self, tasks: Vec<(String, String, String, String, String, String)>) -> PyResult<Vec<bool>> {
        Ok(tasks.par_iter().map(|(i, o, c, r, cr, f)| {
            self.transform(i.clone(), o.clone(), c.clone(), r.clone(), cr.clone(), f.clone()).unwrap_or(false)
        }).collect())
    }
}

#[pymodule]
fn anchorr(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<AnchEngine>()?;
    m.add_class::<VideoMetadata>()?;
    Ok(())
}

