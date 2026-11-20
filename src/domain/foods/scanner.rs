use dioxus::prelude::*;
use icons::X;

use crate::components::ui::field::INPUT;

/// Kamera-Scan läuft im Browser: BarcodeDetector (Android/Chrome) oder ZXing (iOS Safari, Firefox; nachgeladen).
/// Liefert den Code als Rückgabewert, "CANCEL" beim Abbruch oder "ERR:…".
const SCAN_JS: &str = r#"
const video = document.getElementById('nrz-scan-video');
if (!video) return 'ERR:Kein Video';
if (!navigator.mediaDevices || !navigator.mediaDevices.getUserMedia) return 'ERR:Kamera nicht verfügbar (HTTPS nötig)';
let stream;
try {
  stream = await navigator.mediaDevices.getUserMedia({ video: { facingMode: { ideal: 'environment' }, width: { ideal: 1280 }, height: { ideal: 720 } }, audio: false });
} catch (e) { return 'ERR:Kein Zugriff auf die Kamera'; }
video.srcObject = stream;
video.setAttribute('playsinline', 'true');
try { await video.play(); } catch (e) {}
let active = true;
window.__nrzScanStop = () => { active = false; };
const stop = () => { active = false; stream.getTracks().forEach(t => t.stop()); };
const sleep = (ms) => new Promise(r => setTimeout(r, ms));
try {
  let detect = null;
  if ('BarcodeDetector' in window) {
    try {
      const det = new BarcodeDetector({ formats: ['ean_13', 'ean_8', 'upc_a', 'upc_e', 'code_128'] });
      detect = async () => { const r = await det.detect(video); return r.length ? r[0].rawValue : null; };
    } catch (e) { detect = null; }
  }
  if (!detect) {
    if (!window.ZXing) {
      await new Promise((res, rej) => { const s = document.createElement('script'); s.src = '/vendor/zxing.min.js'; s.onload = res; s.onerror = rej; document.head.appendChild(s); });
    }
    const Z = window.ZXing;
    const hints = new Map();
    hints.set(Z.DecodeHintType.POSSIBLE_FORMATS, [Z.BarcodeFormat.EAN_13, Z.BarcodeFormat.EAN_8, Z.BarcodeFormat.UPC_A, Z.BarcodeFormat.UPC_E, Z.BarcodeFormat.CODE_128]);
    hints.set(Z.DecodeHintType.TRY_HARDER, true);
    const reader = new Z.MultiFormatReader();
    reader.setHints(hints);
    const canvas = document.createElement('canvas');
    const ctx = canvas.getContext('2d', { willReadFrequently: true });
    detect = async () => {
      const w = video.videoWidth, h = video.videoHeight;
      if (!w || !h) return null;
      // mittlerer Streifen (dort ist der Rahmen)
      const cw = Math.round(w * 0.8), ch = Math.round(h * 0.45);
      canvas.width = cw; canvas.height = ch;
      ctx.drawImage(video, (w - cw) / 2, (h - ch) / 2, cw, ch, 0, 0, cw, ch);
      try {
        const src = new Z.HTMLCanvasElementLuminanceSource(canvas);
        const bmp = new Z.BinaryBitmap(new Z.HybridBinarizer(src));
        return reader.decode(bmp).getText();
      } catch (e) { return null; }
    };
  }
  while (active) {
    const code = await detect().catch(() => null);
    if (code) { if (navigator.vibrate) navigator.vibrate(40); stop(); return code; }
    await sleep(150);
  }
  stop();
  return 'CANCEL';
} catch (e) { stop(); return 'ERR:' + (e && e.message ? e.message : 'Scan fehlgeschlagen'); }
"#;

/// Vollbild-Scanner mit Rahmen. Manuelle Eingabe als Ausweg.
#[component]
pub fn Scanner(onresult: EventHandler<String>, onclose: EventHandler<()>) -> Element {
    let mut error = use_signal(|| None::<String>);
    let mut manual = use_signal(String::new);

    use_effect(move || {
        spawn(async move {
            match document::eval(SCAN_JS).await {
                Ok(v) => {
                    let s = v.as_str().unwrap_or("").to_string();
                    if let Some(e) = s.strip_prefix("ERR:") {
                        error.set(Some(e.to_string()));
                    } else if !s.is_empty() && s != "CANCEL" {
                        onresult.call(s);
                    }
                }
                Err(e) => error.set(Some(e.to_string())),
            }
        });
    });

    let close = move |_| {
        let _ = document::eval("window.__nrzScanStop && window.__nrzScanStop();");
        onclose.call(());
    };
    let submit_manual = move |_| {
        let code: String = manual().chars().filter(|c| c.is_ascii_digit()).collect();
        if code.len() >= 6 {
            let _ = document::eval("window.__nrzScanStop && window.__nrzScanStop();");
            onresult.call(code);
        }
    };

    rsx! {
        div { class: "anim-fade fixed inset-0 z-[300] flex flex-col bg-black text-white",
            video { id: "nrz-scan-video", class: "absolute inset-0 h-full w-full object-cover", autoplay: true, muted: true, "playsinline": "true" }
            // Abdunklung mit Fenster
            div { class: "pointer-events-none absolute inset-0 flex items-center justify-center",
                div { class: "relative h-40 w-[78%] max-w-sm rounded-3xl shadow-[0_0_0_9999px_rgba(0,0,0,0.55)]",
                    div { class: "absolute inset-0 rounded-3xl border-2 border-white/90" }
                    div { class: "absolute inset-x-6 top-1/2 h-0.5 -translate-y-1/2 animate-pulse rounded-full bg-white/80" }
                }
            }
            div { class: "relative flex items-center justify-between px-4 pt-[calc(env(safe-area-inset-top)+0.75rem)]",
                span { class: "text-base font-semibold drop-shadow", "Barcode scannen" }
                button { class: "flex size-10 items-center justify-center rounded-full bg-white/15 backdrop-blur", onclick: close, X { class: "size-5" } }
            }
            div { class: "flex-1" }
            div { class: "relative flex flex-col gap-3 px-4 pb-[calc(env(safe-area-inset-bottom)+1rem)]",
                if let Some(e) = error() {
                    p { class: "rounded-2xl bg-white/15 px-4 py-3 text-center text-sm backdrop-blur", "{e} – gib den Code unten ein." }
                } else {
                    p { class: "text-center text-sm text-white/80 drop-shadow", "Halte den Strichcode in den Rahmen" }
                }
                div { class: "flex gap-2 rounded-2xl bg-black/50 p-2 backdrop-blur",
                    input {
                        class: "{INPUT} border-white/20 bg-white/10 text-white placeholder:text-white/50",
                        inputmode: "numeric",
                        placeholder: "Code manuell eingeben",
                        value: manual(),
                        oninput: move |e| manual.set(e.value()),
                        onkeydown: move |e| if e.key() == Key::Enter { submit_manual(()) },
                    }
                    button { class: "shrink-0 rounded-xl bg-white px-4 text-sm font-semibold text-black", onclick: move |_| submit_manual(()), "OK" }
                }
            }
        }
    }
}
