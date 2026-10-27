//! Turntable recording request (`viewport.record-orbit`, plugin-api 0.2).
//! The host checks every value here; the UI receives only validated specs.
use super::super::runtime::host::formacad::plugin::viewport::{
    Background, Orbit, UpAxis, VideoFormat,
};
use serde::Serialize;

pub const MAX_FRAMES: u64 = 36_000;

/// Validated recording parameters as sent to the UI (camelCase JSON).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OrbitSpec {
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    pub duration_ms: u32,
    pub turns: f32,
    pub clockwise: bool,
    pub up: &'static str,
    pub tilt_deg: f32,
    pub elevation_deg: f32,
    pub start_deg: f32,
    pub zoom: f32,
    pub background: &'static str,
    pub format: &'static str,
    pub bitrate_kbps: u32,
}

impl OrbitSpec {
    pub fn extension(&self) -> &'static str {
        self.format
    }

    pub fn frames(&self) -> u64 {
        (u64::from(self.duration_ms) * u64::from(self.fps)).div_ceil(1000)
    }
}

fn within(name: &str, value: f32, min: f32, max: f32) -> Result<f32, String> {
    if value.is_finite() && (min..=max).contains(&value) {
        Ok(value)
    } else {
        Err(format!("{name} must be between {min} and {max}"))
    }
}

fn whole(name: &str, value: u32, min: u32, max: u32) -> Result<u32, String> {
    if (min..=max).contains(&value) {
        Ok(value)
    } else {
        Err(format!("{name} must be between {min} and {max}"))
    }
}

pub fn validate(orbit: Orbit) -> Result<OrbitSpec, String> {
    let width = whole("width", orbit.width, 160, 3840)?;
    let height = whole("height", orbit.height, 120, 2160)?;
    if width % 2 != 0 || height % 2 != 0 {
        return Err("width and height must be even".into());
    }
    let spec = OrbitSpec {
        width,
        height,
        fps: whole("fps", orbit.fps, 1, 120)?,
        duration_ms: whole("durationMs", orbit.duration_ms, 500, 600_000)?,
        turns: within("turns", orbit.turns, 0.05, 100.0)?,
        clockwise: orbit.clockwise,
        up: match orbit.up {
            UpAxis::X => "x",
            UpAxis::Y => "y",
            UpAxis::Z => "z",
            UpAxis::MinusX => "-x",
            UpAxis::MinusY => "-y",
            UpAxis::MinusZ => "-z",
        },
        tilt_deg: within("tiltDeg", orbit.tilt_deg, -90.0, 90.0)?,
        elevation_deg: within("elevationDeg", orbit.elevation_deg, -89.0, 89.0)?,
        start_deg: within("startDeg", orbit.start_deg, -360.0, 360.0)?,
        zoom: within("zoom", orbit.zoom, 0.25, 4.0)?,
        background: match orbit.background {
            Background::Theme => "theme",
            Background::Light => "light",
            Background::Dark => "dark",
        },
        format: match orbit.format {
            VideoFormat::Mp4 => "mp4",
            VideoFormat::Webm => "webm",
        },
        bitrate_kbps: match orbit.bitrate_kbps {
            0 => 0,
            value => whole("bitrateKbps", value, 100, 200_000)?,
        },
    };
    if spec.frames() > MAX_FRAMES {
        return Err(format!("A recording is limited to {MAX_FRAMES} frames"));
    }
    Ok(spec)
}

#[cfg(test)]
mod tests {
    use super::*;

    pub fn orbit() -> Orbit {
        Orbit {
            width: 1920,
            height: 1080,
            fps: 30,
            duration_ms: 6000,
            turns: 1.0,
            clockwise: false,
            up: UpAxis::Z,
            tilt_deg: 0.0,
            elevation_deg: 20.0,
            start_deg: 0.0,
            zoom: 1.0,
            background: Background::Theme,
            format: VideoFormat::Mp4,
            bitrate_kbps: 0,
        }
    }

    #[test]
    fn accepts_a_typical_orbit_and_counts_frames() {
        let spec = validate(orbit()).unwrap();
        assert_eq!(spec.frames(), 180);
        assert_eq!(spec.up, "z");
        assert_eq!(spec.extension(), "mp4");
        let json = serde_json::to_value(&spec).unwrap();
        assert_eq!(json["durationMs"], 6000);
        assert_eq!(json["elevationDeg"], 20.0);
    }

    #[test]
    fn rejects_values_out_of_range() {
        let cases: [fn(&mut Orbit); 8] = [
            |o| o.width = 1921,
            |o| o.height = 4000,
            |o| o.fps = 0,
            |o| o.turns = f32::NAN,
            |o| o.elevation_deg = 90.0,
            |o| o.zoom = 10.0,
            |o| o.bitrate_kbps = 5,
            |o| {
                o.fps = 120;
                o.duration_ms = 600_000;
            },
        ];
        for change in cases {
            let mut value = orbit();
            change(&mut value);
            assert!(validate(value).is_err());
        }
    }
}
