use anyhow::{anyhow, Context, Result};
use cleaner_core::{accel::Preference, export::{export_page, Target}, image::decode, patch::Patch, strip::{self, Strip}};
use manga_cleaner_lib::run::{model_search_paths, Cleaner, PageContext, Pipeline, RegionOutcome, HIGHEST_LOCAL};
use std::{env, fs, path::{Path, PathBuf}, time::Instant};

fn main() -> Result<()> {
    let mut args = env::args().skip(1);
    let input = PathBuf::from(args.next().ok_or_else(|| anyhow!("usage: bear-runner INPUT_DIR OUTPUT_DIR"))?);
    let output = PathBuf::from(args.next().ok_or_else(|| anyhow!("usage: bear-runner INPUT_DIR OUTPUT_DIR"))?);
    if args.next().is_some() { return Err(anyhow!("too many arguments")); }
    fs::create_dir_all(&output).context("create output directory")?;
    let models = env::var_os("BEAR_MODELS_DIR").map(PathBuf::from)
        .or_else(|| model_search_paths(None).into_iter().find(|p| p.join("comictextdetector.onnx").exists()))
        .ok_or_else(|| anyhow!("model directory not found; set BEAR_MODELS_DIR"))?;
    let ort = env::var_os("BEAR_ORT_PATH").map(PathBuf::from)
        .or_else(|| cleaner_core::runtime::find(None).ok())
        .ok_or_else(|| anyhow!("ONNX Runtime not found; set BEAR_ORT_PATH"))?;
    cleaner_core::runtime::load(&ort).context("load ONNX Runtime")?;
    let mut paths = fs::read_dir(&input)?.filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| is_image(p)).collect::<Vec<_>>();
    paths.sort();
    if paths.is_empty() { return Err(anyhow!("no supported images found")); }
    let sources = paths.iter().map(fs::read).collect::<std::io::Result<Vec<_>>>()?;
    let rasters = sources.iter().map(|b| decode(b).map_err(|e| anyhow!("{e}"))).collect::<Result<Vec<_>>>()?;
    let sizes = rasters.iter().map(|r| (r.width, r.height)).collect::<Vec<_>>();
    let strip = Strip::of_sizes(&sizes);
    let survey = strip::survey(&strip, |_| None);
    let started = Instant::now();
    let mut pipeline = Pipeline::open(&models, Preference::Automatic).map_err(|e| anyhow!("open Bear pipeline: {e}"))?;
    for (index, (path, bytes)) in paths.iter().zip(sources.iter()).enumerate() {
        let context = PageContext { strip: &strip, joins: &survey.joins, segments: &survey.segments, placement: index, sources: &[] };
        let page_id = format!("page-{:04}", index + 1);
        let outcome = pipeline.clean_page(&page_id, bytes, HIGHEST_LOCAL, &context)
            .map_err(|e| anyhow!("{}: {e}", path.display()))?;
        let mut patches: Vec<Patch> = Vec::new();
        let mut cleaned = 0usize;
        let mut untouched = 0usize;
        for region in &outcome.regions {
            match region {
                RegionOutcome::Cleaned(patch, _) => { patches.push((**patch).clone()); cleaned += 1; }
                RegionOutcome::Untouched { .. } | RegionOutcome::Candidate { .. } | RegionOutcome::Declined { .. } => untouched += 1,
                RegionOutcome::Detected(_) => return Err(anyhow!("unexpected detect-only result")),
            }
        }
        let export = export_page(bytes, &patches, Target::SameAsSource).map_err(|e| anyhow!("export: {e}"))?;
        let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("page");
        fs::write(output.join(format!("{:04}-{}.cleaned.png", index + 1, stem)), export.bytes)?;
        println!("PROGRESS {}/{} cleaned={} untouched={}", index + 1, paths.len(), cleaned, untouched);
    }
    println!("DONE pages={} elapsed_seconds={:.2}", paths.len(), started.elapsed().as_secs_f64());
    Ok(())
}
fn is_image(path: &Path) -> bool {
    matches!(path.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase().as_str(), "png" | "jpg" | "jpeg" | "webp" | "tif" | "tiff" | "bmp")
}
