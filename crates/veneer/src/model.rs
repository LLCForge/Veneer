use serde::{Deserialize, Serialize};

/// Rendering engine family. Decides vendor strings, masked WebGL values and which APIs exist.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Engine {
    Chromium,
    Gecko,
    Webkit,
}

/// Device form factor.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Desktop,
    Laptop,
    Phone,
    Tablet,
    Console,
    Tv,
    Vr,
    /// Handheld gaming PC (Steam Deck, ROG Ally): desktop browsers, touch from device data.
    Handheld,
}

impl Kind {
    /// Phones and tablets use the mobile toolbar and must report touch points.
    pub fn is_handheld(self) -> bool {
        matches!(self, Kind::Phone | Kind::Tablet)
    }
}

/// What the caller wants. Every field is optional; ids come from the embedded dataset.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Constraints {
    /// Operating system id, e.g. `windows`, `macos`, `android`, `ios`, `playstation`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub os: Option<String>,
    /// Browser id, e.g. `chrome`, `edge`, `firefox`, `safari`, `samsung`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub browser: Option<String>,
    /// Device form factor.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<Kind>,
    /// Exact device id, e.g. `iphone-15` or `mac-macbook-air-m2`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub device: Option<String>,
    /// Rendering engine, e.g. to match the Playwright browser you launch.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub engine: Option<Engine>,
    /// Case-insensitive match against the GPU vendor or renderer, e.g. `nvidia`, `apple`, `adreno`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gpu_vendor: Option<String>,
    /// Lowest browser major version.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_version: Option<u32>,
    /// Highest browser major version.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_version: Option<u32>,
    /// BCP 47 locale (`tr-TR`), bare language (`tr`), or a comma-separated list of either (`tr,en`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub locale: Option<String>,
    /// ISO 3166 country code; picks a locale of that region, e.g. `TR`, `US`, `DE`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub country: Option<String>,
    /// IANA timezone, e.g. `Europe/Istanbul`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timezone: Option<String>,
    /// Any string. The same seed and constraints always produce the same profile.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seed: Option<String>,
    /// Smallest `screen.width` in CSS pixels.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_width: Option<u32>,
    /// Largest `screen.width` in CSS pixels.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_width: Option<u32>,
    /// Smallest `screen.height` in CSS pixels.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_height: Option<u32>,
    /// Largest `screen.height` in CSS pixels.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_height: Option<u32>,
    /// Smallest `devicePixelRatio`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_dpr: Option<f64>,
    /// Largest `devicePixelRatio`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_dpr: Option<f64>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Profile {
    pub seed: String,
    pub device: DeviceInfo,
    pub os: OsInfo,
    pub browser: BrowserInfo,
    pub navigator: Navigator,
    pub screen: Screen,
    pub viewport: Viewport,
    pub webgl: WebGl,
    pub canvas: Noise,
    pub audio: Noise,
    pub fonts: Vec<String>,
    pub locale: String,
    pub timezone: String,
    /// Fields contributed by extension modules (`src/extras`), flattened into the profile JSON.
    #[serde(flatten, default)]
    pub ext: crate::extras::Extensions,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceInfo {
    pub id: String,
    pub kind: Kind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OsInfo {
    pub id: String,
    pub name: String,
    pub version: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowserInfo {
    pub id: String,
    pub name: String,
    pub engine: Engine,
    pub version: String,
    pub major: u32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Navigator {
    pub user_agent: String,
    pub app_version: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_agent_data: Option<UserAgentData>,
    pub platform: String,
    pub vendor: String,
    pub language: String,
    pub languages: Vec<String>,
    pub hardware_concurrency: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub device_memory: Option<f64>,
    pub max_touch_points: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub oscpu: Option<String>,
}

/// User-Agent Client Hints, as returned by `navigator.userAgentData`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserAgentData {
    pub brands: Vec<Brand>,
    pub full_version_list: Vec<Brand>,
    pub mobile: bool,
    pub platform: String,
    pub platform_version: String,
    pub architecture: String,
    pub bitness: String,
    pub model: String,
    pub ua_full_version: String,
    pub form_factors: Vec<String>,
    pub wow64: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Brand {
    pub brand: String,
    pub version: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Screen {
    pub width: u32,
    pub height: u32,
    pub avail_width: u32,
    pub avail_height: u32,
    pub color_depth: u32,
    pub pixel_depth: u32,
    pub device_pixel_ratio: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Viewport {
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WebGl {
    pub vendor: String,
    pub renderer: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unmasked_vendor: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unmasked_renderer: Option<String>,
}

/// Seed for deterministic canvas or audio noise.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Noise {
    pub seed: u32,
}
