// Draws the label artwork as ID textures for the Blender packaging scenes.
// Channels: R = ink coverage, G = reserved (accent), B = 0. Anything else is "label paper".
// Usage: swiftc -O make_labels.swift -o /tmp/make_labels && /tmp/make_labels <out_dir>
import Cocoa
import CoreText

func makeContext(_ w: Int, _ h: Int) -> CGContext {
    let cs = CGColorSpaceCreateDeviceRGB()
    let ctx = CGContext(data: nil, width: w, height: h, bitsPerComponent: 8, bytesPerRow: 0, space: cs,
                        bitmapInfo: CGImageAlphaInfo.noneSkipLast.rawValue)!
    ctx.setFillColor(CGColor(red: 0, green: 0, blue: 0, alpha: 1))
    ctx.fill(CGRect(x: 0, y: 0, width: w, height: h))
    ctx.setShouldAntialias(true)
    ctx.setAllowsFontSmoothing(false)
    ctx.setShouldSmoothFonts(false)
    return ctx
}

let ink = CGColor(red: 1, green: 0, blue: 0, alpha: 1)

/// Draws `s` centered on x, with its baseline at y (y measured from the top of the image).
func text(_ ctx: CGContext, _ s: String, font: String, size: CGFloat, kern: CGFloat, cx: CGFloat, top: CGFloat, h: CGFloat) {
    let f = CTFontCreateWithName(font as CFString, size, nil)
    let attrs: [NSAttributedString.Key: Any] = [
        .font: f, .kern: kern,
        .foregroundColor: ink,
    ]
    let line = CTLineCreateWithAttributedString(NSAttributedString(string: s, attributes: attrs))
    let bounds = CTLineGetBoundsWithOptions(line, [])
    let width = bounds.width - kern // trailing kern isn't visible
    ctx.textPosition = CGPoint(x: cx - width / 2 - bounds.minX, y: h - top - size * 0.72)
    CTLineDraw(line, ctx)
}

func logo(_ ctx: CGContext, cx: CGFloat, cy: CGFloat, r: CGFloat, h: CGFloat) {
    let cy = h - cy
    ctx.setStrokeColor(ink)
    ctx.setFillColor(ink)
    ctx.setLineWidth(r * 0.075)
    ctx.strokeEllipse(in: CGRect(x: cx - r, y: cy - r, width: r * 2, height: r * 2))
    ctx.saveGState()
    ctx.addEllipse(in: CGRect(x: cx - r * 0.93, y: cy - r * 0.93, width: r * 1.86, height: r * 1.86))
    ctx.clip()
    // Two peaks.
    ctx.move(to: CGPoint(x: cx - r * 0.82, y: cy - r * 0.18))
    ctx.addLine(to: CGPoint(x: cx - r * 0.30, y: cy + r * 0.38))
    ctx.addLine(to: CGPoint(x: cx + r * 0.10, y: cy - r * 0.18))
    ctx.closePath()
    ctx.fillPath()
    ctx.move(to: CGPoint(x: cx - r * 0.12, y: cy - r * 0.18))
    ctx.addLine(to: CGPoint(x: cx + r * 0.32, y: cy + r * 0.52))
    ctx.addLine(to: CGPoint(x: cx + r * 0.82, y: cy - r * 0.18))
    ctx.closePath()
    ctx.fillPath()
    // Water lines.
    for i in 0..<3 {
        let y = cy - r * (0.30 + CGFloat(i) * 0.22)
        ctx.fill(CGRect(x: cx - r, y: y, width: r * 2, height: r * 0.075))
    }
    ctx.restoreGState()
}

func save(_ ctx: CGContext, _ path: String) {
    let rep = NSBitmapImageRep(cgImage: ctx.makeImage()!)
    try! rep.representation(using: .png, properties: [:])!.write(to: URL(fileURLWithPath: path))
}

let out = CommandLine.arguments.count > 1 ? CommandLine.arguments[1] : "."

// ---- Bottle: 4096 x 1600, content centred at x = 2048, kept within about +-520 px.
do {
    let w = 4096, h = 1600
    let c = makeContext(w, h)
    let cx = CGFloat(w) / 2
    text(c, "Northwind", font: "AvenirNext-DemiBold", size: 215, kern: -2, cx: cx, top: 150, h: CGFloat(h))
    text(c, "EVERYDAY WELLNESS", font: "AvenirNext-Medium", size: 52, kern: 13, cx: cx, top: 400, h: CGFloat(h))
    text(c, "NATURALLY", font: "AvenirNext-Medium", size: 52, kern: 13, cx: cx, top: 470, h: CGFloat(h))
    logo(c, cx: cx, cy: 760, r: 190, h: CGFloat(h))
    text(c, "ALPINE MINERALS", font: "AvenirNext-DemiBold", size: 84, kern: 10, cx: cx, top: 1040, h: CGFloat(h))
    text(c, "BODY WASH", font: "AvenirNext-DemiBold", size: 84, kern: 10, cx: cx, top: 1145, h: CGFloat(h))
    text(c, "CLEANSE  /  REFRESH  /  RECHARGE", font: "AvenirNext-Medium", size: 40, kern: 7, cx: cx, top: 1300, h: CGFloat(h))
    text(c, "500 mL   e   16.9 FL OZ", font: "AvenirNext-Medium", size: 42, kern: 5, cx: cx, top: 1440, h: CGFloat(h))
    save(c, "\(out)/label_bottle.png")
}

// ---- Jar: 4096 x 640.
do {
    let w = 4096, h = 640
    let c = makeContext(w, h)
    let cx = CGFloat(w) / 2
    text(c, "Northwind", font: "AvenirNext-DemiBold", size: 190, kern: -2, cx: cx, top: 70, h: CGFloat(h))
    text(c, "NOURISHING FACE CREAM", font: "AvenirNext-DemiBold", size: 62, kern: 10, cx: cx, top: 330, h: CGFloat(h))
    text(c, "HYDRATE  /  PROTECT  /  BALANCE", font: "AvenirNext-Medium", size: 42, kern: 8, cx: cx, top: 440, h: CGFloat(h))
    text(c, "50 mL   e   1.7 FL OZ", font: "AvenirNext-Medium", size: 42, kern: 5, cx: cx, top: 540, h: CGFloat(h))
    save(c, "\(out)/label_jar.png")
}
print("ok")
