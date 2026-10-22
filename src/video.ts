import { invoke } from "@tauri-apps/api/core";

// What OpenH264 on the host produces: Constrained Baseline, level 4.0
// (1080p30), Annex-B byte stream without B-frames.
export const H264_CODEC = "avc1.42C028";

// The video codecs this webview can decode through WebCodecs. WebKitGTK and
// older WKWebView builds lack VideoDecoder; they keep the JPEG tile stream.
async function supportedDecoders(): Promise<string[]> {
	if (typeof VideoDecoder === "undefined") {
		return [];
	}
	try {
		const { supported } = await VideoDecoder.isConfigSupported({
			codec: H264_CODEC,
			optimizeForLatency: true,
		});
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
