import { invoke } from "@tauri-apps/api/core";

// What OpenH264 on the host produces: Constrained Baseline, level 4.0
// (1080p30), Annex-B byte stream without B-frames.
// WebView2's hardware (D3D11) decoder shows full-width horizontal flash lines
// on Windows; its software decoder does not, and one stream is cheap to decode.
const isWindows = navigator.userAgent.includes("Windows");
export const H264_CONFIG: VideoDecoderConfig = {
	codec: "avc1.42C028",
	optimizeForLatency: true,
	hardwareAcceleration: isWindows ? "prefer-software" : "no-preference",
};

// The video codecs this webview can decode through WebCodecs. WebKitGTK and
// older WKWebView builds lack VideoDecoder; they keep the JPEG tile stream.
async function supportedDecoders(): Promise<string[]> {
	if (typeof VideoDecoder === "undefined") {
		return [];
	}
	try {
		const { supported } = await VideoDecoder.isConfigSupported(H264_CONFIG);
		return supported ? ["h264"] : [];
	} catch {
		return [];
	}
}

// Tell the backend once per window; it offers hosts only what decodes here.
export function reportDecoders(): void {
	void supportedDecoders()
		.then((codecs) => invoke("lan_set_decoders", { codecs }))
		.catch(() => { /* backend gone */ });
}
