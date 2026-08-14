use crate::models::DeviceProfile;
use std::collections::HashMap;

pub struct DeviceManager;

impl DeviceManager {
    /// Pixel 7a (Lynx, Android 14) default profile
    pub fn pixel_7a() -> DeviceProfile {
        DeviceProfile {
            name: "Google Pixel 7a".to_string(),
            fingerprint: "google/lynx/lynx:14/UQ1A.240105.004/11269997:user/release-keys"
                .to_string(),
            hardware: "lynx".to_string(),
            radio: "g5300g-231013-231106-B-11082531".to_string(),
            brand: "google".to_string(),
            device: "lynx".to_string(),
            sdk_int: 34,
            release: "14".to_string(),
            model: "Pixel 7a".to_string(),
            manufacturer: "Google".to_string(),
            product: "lynx".to_string(),
            id: "UQ1A.240105.004".to_string(),
            bootloader: "lynx-1.2-10651717".to_string(),
            density: 420,
            width: 1080,
            height: 2400,
            platforms: vec![
                "arm64-v8a".to_string(),
                "armeabi-v7a".to_string(),
                "armeabi".to_string(),
            ],
            features: vec![
                "android.hardware.audio.low_latency".to_string(),
                "android.hardware.audio.output".to_string(),
                "android.hardware.audio.pro".to_string(),
                "android.hardware.bluetooth".to_string(),
                "android.hardware.bluetooth_le".to_string(),
                "android.hardware.camera".to_string(),
                "android.hardware.camera.any".to_string(),
                "android.hardware.camera.autofocus".to_string(),
                "android.hardware.camera.capability.manual_post_processing".to_string(),
                "android.hardware.camera.capability.manual_sensor".to_string(),
                "android.hardware.camera.capability.raw".to_string(),
                "android.hardware.camera.concurrent".to_string(),
                "android.hardware.camera.flash".to_string(),
                "android.hardware.camera.front".to_string(),
                "android.hardware.camera.level.full".to_string(),
                "android.hardware.faketouch".to_string(),
                "android.hardware.fingerprint".to_string(),
                "android.hardware.biometrics.face".to_string(),
                "android.hardware.location".to_string(),
                "android.hardware.location.gps".to_string(),
                "android.hardware.location.network".to_string(),
                "android.hardware.microphone".to_string(),
                "android.hardware.nfc".to_string(),
                "android.hardware.nfc.any".to_string(),
                "android.hardware.nfc.hce".to_string(),
                "android.hardware.nfc.hcef".to_string(),
                "android.hardware.nfc.uicc".to_string(),
                "android.hardware.screen.landscape".to_string(),
                "android.hardware.screen.portrait".to_string(),
                "android.hardware.security.model.compatible".to_string(),
                "android.hardware.sensor.accelerometer".to_string(),
                "android.hardware.sensor.barometer".to_string(),
                "android.hardware.sensor.compass".to_string(),
                "android.hardware.sensor.dynamic.head_tracker".to_string(),
                "android.hardware.sensor.gyroscope".to_string(),
                "android.hardware.sensor.hifi_sensors".to_string(),
                "android.hardware.sensor.light".to_string(),
                "android.hardware.sensor.proximity".to_string(),
                "android.hardware.sensor.relative_humidity".to_string(),
                "android.hardware.sensor.stepcounter".to_string(),
                "android.hardware.sensor.stepdetector".to_string(),
                "android.hardware.telephony".to_string(),
                "android.hardware.telephony.calling".to_string(),
                "android.hardware.telephony.cdma".to_string(),
                "android.hardware.telephony.data".to_string(),
                "android.hardware.telephony.gsm".to_string(),
                "android.hardware.telephony.ims".to_string(),
                "android.hardware.telephony.mbms".to_string(),
                "android.hardware.telephony.radio.access".to_string(),
                "android.hardware.touchscreen".to_string(),
                "android.hardware.touchscreen.multitouch".to_string(),
                "android.hardware.touchscreen.multitouch.distinct".to_string(),
                "android.hardware.touchscreen.multitouch.jazzhand".to_string(),
                "android.hardware.usb.accessory".to_string(),
                "android.hardware.usb.host".to_string(),
                "android.hardware.vulkan.compute".to_string(),
                "android.hardware.vulkan.level".to_string(),
                "android.hardware.vulkan.version".to_string(),
                "android.hardware.wifi".to_string(),
                "android.hardware.wifi.aware".to_string(),
                "android.hardware.wifi.direct".to_string(),
                "android.hardware.wifi.passpoint".to_string(),
                "android.hardware.wifi.rtt".to_string(),
                "android.software.activities_on_secondary_displays".to_string(),
                "android.software.app_widgets".to_string(),
                "android.software.autofill".to_string(),
                "android.software.backup".to_string(),
                "android.software.cant_save_state".to_string(),
                "android.software.companion_device_setup".to_string(),
                "android.software.connections_service".to_string(),
                "android.software.controls".to_string(),
                "android.software.cts".to_string(),
                "android.software.device_admin".to_string(),
                "android.software.expanded_picture_in_picture".to_string(),
                "android.software.file_based_encryption".to_string(),
                "android.software.freeform_window_management".to_string(),
                "android.software.home_screen".to_string(),
                "android.software.input_methods".to_string(),
                "android.software.ipsec_tunnels".to_string(),
                "android.software.live_tv".to_string(),
                "android.software.live_wallpaper".to_string(),
                "android.software.managed_users".to_string(),
                "android.software.midi".to_string(),
                "android.software.opengles.deqp.level".to_string(),
                "android.software.picture_in_picture".to_string(),
                "android.software.print".to_string(),
                "android.software.secure_lock_screen".to_string(),
                "android.software.securely_removes_users".to_string(),
                "android.software.sip".to_string(),
                "android.software.sip.voip".to_string(),
                "android.software.verified_boot".to_string(),
                "android.software.voice_recognizers".to_string(),
                "android.software.vulkan.deqp.level".to_string(),
                "android.software.webview".to_string(),
            ],
            locales: vec![
                "en_US".to_string(),
                "en_GB".to_string(),
                "en".to_string(),
                "km_KH".to_string(),
                "km".to_string(),
                "fr_FR".to_string(),
                "es_ES".to_string(),
                "de_DE".to_string(),
                "zh_CN".to_string(),
                "ja_JP".to_string(),
            ],
            shared_libraries: vec![
                "android.ext.services".to_string(),
                "android.ext.shared".to_string(),
                "android.hidl.base-V1.0-java".to_string(),
                "android.hidl.manager-V1.0-java".to_string(),
                "android.net.ipsec.ike".to_string(),
                "android.test.base".to_string(),
                "android.test.mock".to_string(),
                "android.test.runner".to_string(),
                "com.android.future.usb.accessory".to_string(),
                "com.android.location.provider".to_string(),
                "com.android.media.remotedisplay".to_string(),
                "com.android.mediadrm.signer".to_string(),
                "com.android.nfc_extras".to_string(),
                "com.google.android.camera.experimental2017".to_string(),
                "com.google.android.dialer.support".to_string(),
                "com.google.android.maps".to_string(),
                "com.google.android.media.effects".to_string(),
                "javax.obex".to_string(),
                "org.apache.http.legacy".to_string(),
            ],
            gl_version: 196610,
            gl_extensions: vec![
                "GL_EXT_debug_marker".to_string(),
                "GL_EXT_discard_framebuffer".to_string(),
                "GL_EXT_robustness".to_string(),
                "GL_EXT_texture_format_BGRA8888".to_string(),
                "GL_OES_compressed_ETC1_RGB8_texture".to_string(),
                "GL_OES_depth_texture".to_string(),
                "GL_OES_depth24".to_string(),
                "GL_OES_EGL_image".to_string(),
                "GL_OES_EGL_image_external".to_string(),
                "GL_OES_EGL_sync".to_string(),
                "GL_OES_element_index_uint".to_string(),
                "GL_OES_fbo_render_mipmap".to_string(),
                "GL_OES_get_program_binary".to_string(),
                "GL_OES_packed_depth_stencil".to_string(),
                "GL_OES_rgb8_rgba8".to_string(),
                "GL_OES_standard_derivatives".to_string(),
                "GL_OES_texture_float".to_string(),
                "GL_OES_texture_half_float".to_string(),
                "GL_OES_texture_npot".to_string(),
                "GL_OES_vertex_array_object".to_string(),
                "GL_OES_vertex_half_float".to_string(),
            ],
            gsf_version: 240415037,
            vending_version: 83933000,
            vending_version_string: "39.3.30-29 [0] [PR] 603176503".to_string(),
            client: "android-google".to_string(),
            roaming: "mobile-notroaming".to_string(),
            timezone: "UTC".to_string(),
            cell_operator: "310".to_string(),
            sim_operator: "38".to_string(),
        }
    }

    /// Samsung Galaxy S23 (Android 14) profile
    #[allow(dead_code)]
    pub fn samsung_s23() -> DeviceProfile {
        let pixel_7a = Self::pixel_7a();
        DeviceProfile {
            name: "Samsung Galaxy S23".to_string(),
            fingerprint: "samsung/dm1quew/dm1q:14/UP1A.231005.007/S911BXXU3BWK5:user/release-keys"
                .to_string(),
            hardware: "qcom".to_string(),
            radio: "S911BXXU3BWK5".to_string(),
            brand: "samsung".to_string(),
            device: "dm1q".to_string(),
            sdk_int: 34,
            release: "14".to_string(),
            model: "SM-S911B".to_string(),
            manufacturer: "samsung".to_string(),
            product: "dm1qxeea".to_string(),
            id: "UP1A.231005.007".to_string(),
            bootloader: "S911BXXU3BWK5".to_string(),
            density: 480,
            width: 1080,
            height: 2340,
            platforms: vec![
                "arm64-v8a".to_string(),
                "armeabi-v7a".to_string(),
                "armeabi".to_string(),
            ],
            features: pixel_7a.features,
            locales: pixel_7a.locales,
            shared_libraries: pixel_7a.shared_libraries,
            gl_version: pixel_7a.gl_version,
            gl_extensions: pixel_7a.gl_extensions,
            gsf_version: 240415037,
            vending_version: 83933000,
            vending_version_string: "39.3.30-29 [0] [PR] 603176503".to_string(),
            client: "android-google".to_string(),
            roaming: "mobile-notroaming".to_string(),
            timezone: "UTC".to_string(),
            cell_operator: "310".to_string(),
            sim_operator: "38".to_string(),
        }
    }

    /// Converts the device profile into the JSON Map format expected by the Aurora dispenser
    pub fn to_dispenser_payload(profile: &DeviceProfile) -> HashMap<String, String> {
        let mut map = HashMap::new();
        map.insert("UserReadableName".to_string(), profile.name.clone());
        map.insert("Build.HARDWARE".to_string(), profile.hardware.clone());
        map.insert("Build.RADIO".to_string(), profile.radio.clone());
        map.insert("Build.FINGERPRINT".to_string(), profile.fingerprint.clone());
        map.insert("Build.BRAND".to_string(), profile.brand.clone());
        map.insert("Build.DEVICE".to_string(), profile.device.clone());
        map.insert(
            "Build.VERSION.SDK_INT".to_string(),
            profile.sdk_int.to_string(),
        );
        map.insert("Build.VERSION.RELEASE".to_string(), profile.release.clone());
        map.insert("Build.MODEL".to_string(), profile.model.clone());
        map.insert(
            "Build.MANUFACTURER".to_string(),
            profile.manufacturer.clone(),
        );
        map.insert("Build.PRODUCT".to_string(), profile.product.clone());
        map.insert("Build.ID".to_string(), profile.id.clone());
        map.insert("Build.BOOTLOADER".to_string(), profile.bootloader.clone());
        map.insert("TouchScreen".to_string(), "3".to_string());
        map.insert("Keyboard".to_string(), "1".to_string());
        map.insert("Navigation".to_string(), "1".to_string());
        map.insert("ScreenLayout".to_string(), "2".to_string());
        map.insert("HasHardKeyboard".to_string(), "false".to_string());
        map.insert("HasFiveWayNavigation".to_string(), "false".to_string());
        map.insert("Screen.Density".to_string(), profile.density.to_string());
        map.insert("Screen.Width".to_string(), profile.width.to_string());
        map.insert("Screen.Height".to_string(), profile.height.to_string());
        map.insert("Platforms".to_string(), profile.platforms.join(","));
        map.insert("Features".to_string(), profile.features.join(","));
        map.insert("Locales".to_string(), profile.locales.join(","));
        map.insert(
            "SharedLibraries".to_string(),
            profile.shared_libraries.join(","),
        );
        map.insert("GL.Version".to_string(), profile.gl_version.to_string());
        map.insert("GL.Extensions".to_string(), profile.gl_extensions.join(","));
        map.insert("Client".to_string(), profile.client.clone());
        map.insert("GSF.version".to_string(), profile.gsf_version.to_string());
        map.insert(
            "Vending.version".to_string(),
            profile.vending_version.to_string(),
        );
        map.insert(
            "Vending.versionString".to_string(),
            profile.vending_version_string.clone(),
        );
        map.insert("Roaming".to_string(), profile.roaming.clone());
        map.insert("TimeZone".to_string(), profile.timezone.clone());
        map.insert("CellOperator".to_string(), profile.cell_operator.clone());
        map.insert("SimOperator".to_string(), profile.sim_operator.clone());
        map
    }
}
