// Artwork for the pouch, box and tote Blender scenes, as exact coverage partitions:
//   R = ink, G = accent, B = paper (the "surface" panel), none of them = the product's body color.
// The regions never overlap, so ink + accent + paper <= 1 and the leftover is body.
// Usage: swiftc -O make_art.swift -o /tmp/make_art && /tmp/make_art <out_dir>
import Cocoa
import CoreText

let INK = CGColor(red: 1, green: 0, blue: 0, alpha: 1)
let ACC = CGColor(red: 0, green: 1, blue: 0, alpha: 1)
let PAPER = CGColor(red: 0, green: 0, blue: 1, alpha: 1)

func makeContext(_ w: Int, _ h: Int) -> CGContext {
    let ctx = CGContext(data: nil, width: w, height: h, bitsPerComponent: 8, bytesPerRow: 0,
                        space: CGColorSpaceCreateDeviceRGB(), bitmapInfo: CGImageAlphaInfo.noneSkipLast.rawValue)!
    ctx.setFillColor(CGColor(red: 0, green: 0, blue: 0, alpha: 1))
    ctx.fill(CGRect(x: 0, y: 0, width: w, height: h))
    ctx.setShouldAntialias(true)
    ctx.setAllowsFontSmoothing(false)
    ctx.setShouldSmoothFonts(false)
    return ctx
}

struct Art {
    let c: CGContext
    let w: CGFloat
    let h: CGFloat
    init(_ w: Int, _ h: Int) { c = makeContext(w, h); self.w = CGFloat(w); self.h = CGFloat(h) }

    func rect(_ x: CGFloat, _ y: CGFloat, _ rw: CGFloat, _ rh: CGFloat, _ col: CGColor, radius: CGFloat = 0) {
        c.setFillColor(col)
        let r = CGRect(x: x, y: h - y - rh, width: rw, height: rh)
        if radius > 0 {
            c.addPath(CGPath(roundedRect: r, cornerWidth: radius, cornerHeight: radius, transform: nil))
            c.fillPath()
        } else {
            c.fill(r)
        }
    }

    func text(_ s: String, _ font: String, _ size: CGFloat, kern: CGFloat, cx: CGFloat, top: CGFloat, _ col: CGColor = INK) {
        let f = CTFontCreateWithName(font as CFString, size, nil)
        let attrs: [NSAttributedString.Key: Any] = [.font: f, .kern: kern, .foregroundColor: col]
        let line = CTLineCreateWithAttributedString(NSAttributedString(string: s, attributes: attrs))
        let b = CTLineGetBoundsWithOptions(line, [])
        c.textPosition = CGPoint(x: cx - (b.width - kern) / 2 - b.minX, y: h - top - size * 0.72)
        CTLineDraw(line, c)
    }

    func ring(_ cx: CGFloat, _ cy: CGFloat, _ r: CGFloat, _ lw: CGFloat, _ col: CGColor = INK) {
        c.setStrokeColor(col)
        c.setLineWidth(lw)
        c.strokeEllipse(in: CGRect(x: cx - r, y: h - cy - r, width: r * 2, height: r * 2))
    }

    /// Mountains-over-water mark inside a ring.
    func logo(_ cx: CGFloat, _ cy: CGFloat, _ r: CGFloat) {
        let y = h - cy
        c.setStrokeColor(INK)
        c.setFillColor(INK)
        c.setLineWidth(r * 0.075)
        c.strokeEllipse(in: CGRect(x: cx - r, y: y - r, width: r * 2, height: r * 2))
        c.saveGState()
        c.addEllipse(in: CGRect(x: cx - r * 0.93, y: y - r * 0.93, width: r * 1.86, height: r * 1.86))
        c.clip()
        c.move(to: CGPoint(x: cx - r * 0.82, y: y - r * 0.18)); c.addLine(to: CGPoint(x: cx - r * 0.30, y: y + r * 0.38))
        c.addLine(to: CGPoint(x: cx + r * 0.10, y: y - r * 0.18)); c.closePath(); c.fillPath()
        c.move(to: CGPoint(x: cx - r * 0.12, y: y - r * 0.18)); c.addLine(to: CGPoint(x: cx + r * 0.32, y: y + r * 0.52))
        c.addLine(to: CGPoint(x: cx + r * 0.82, y: y - r * 0.18)); c.closePath(); c.fillPath()
        for i in 0..<3 { c.fill(CGRect(x: cx - r, y: y - r * (0.30 + CGFloat(i) * 0.22), width: r * 2, height: r * 0.075)) }
        c.restoreGState()
    }

    /// Open mountain mark (no ring) with wind lines.
    func peaks(_ cx: CGFloat, _ cy: CGFloat, _ s: CGFloat) {
        let y = h - cy
        c.setFillColor(INK)
        c.move(to: CGPoint(x: cx - s, y: y - s * 0.35)); c.addLine(to: CGPoint(x: cx - s * 0.35, y: y + s * 0.45))
        c.addLine(to: CGPoint(x: cx + s * 0.05, y: y - s * 0.05)); c.addLine(to: CGPoint(x: cx + s * 0.40, y: y + s * 0.55))
        c.addLine(to: CGPoint(x: cx + s * 0.95, y: y - s * 0.35)); c.addLine(to: CGPoint(x: cx + s * 0.55, y: y - s * 0.35))
        c.addLine(to: CGPoint(x: cx + s * 0.40, y: y - s * 0.12)); c.addLine(to: CGPoint(x: cx + s * 0.05, y: y - s * 0.35))
        c.addLine(to: CGPoint(x: cx - s * 0.35, y: y + s * 0.10)); c.addLine(to: CGPoint(x: cx - s * 0.65, y: y - s * 0.35))
        c.closePath(); c.fillPath()
    }

    func zigzag(_ ys: [CGFloat], _ lw: CGFloat, _ col: CGColor = ACC) {
        c.setStrokeColor(col)
        c.setLineWidth(lw)
        c.setLineJoin(.miter)
        c.setMiterLimit(8)
        let n = ys.count
        for (i, yy) in ys.enumerated() {
            let p = CGPoint(x: w * CGFloat(i) / CGFloat(n - 1), y: h - yy)
            if i == 0 { c.move(to: p) } else { c.addLine(to: p) }
        }
        c.strokePath()
    }

    func save(_ path: String) {
        let rep = NSBitmapImageRep(cgImage: c.makeImage()!)
        try! rep.representation(using: .png, properties: [:])!.write(to: URL(fileURLWithPath: path))
    }
}

let out = CommandLine.arguments.count > 1 ? CommandLine.arguments[1] : "."
let DEMI = "AvenirNext-DemiBold", MED = "AvenirNext-Medium", BOLD = "AvenirNext-Bold"

// ---------------- Pouch front: 2048 x 2976 ----------------
do {
    let a = Art(2048, 2976)
    let cx: CGFloat = 1024
    a.peaks(cx, 560, 150)
    a.text("Northwind", DEMI, 240, kern: -3, cx: cx, top: 700)
    // Zigzag chevrons first so the label sits above them.
    a.zigzag([2760, 2520, 2780, 2520, 2760], 150)
    a.zigzag([3020, 2780, 3040, 2780, 3020], 80)
    a.rect(230, 1010, 1588, 1330, PAPER, radius: 130)
    a.text("PLANT PROTEIN", BOLD, 150, kern: 7, cx: cx, top: 1090)
    a.text("POWDER", BOLD, 150, kern: 7, cx: cx, top: 1260)
    a.text("CLEAN NUTRITION", MED, 48, kern: 10, cx: cx, top: 1470)
    a.text("FOR A BRIGHTER TOMORROW", MED, 48, kern: 10, cx: cx, top: 1540)
    a.rect(780, 1650, 488, 14, ACC)
    for (i, cap) in ["PLANT BASED", "24G PROTEIN", "NO ADDED SUGAR"].enumerated() {
        let x = 480 + CGFloat(i) * 544
        a.ring(x, 1820, 66, 7)
        a.text(cap, MED, 34, kern: 4, cx: x, top: 1920)
    }
    a.rect(748, 1750, 6, 200, INK)
    a.rect(1292, 1750, 6, 200, INK)
    a.text("VANILLA", DEMI, 66, kern: 12, cx: cx, top: 2110)
    a.text("NET WT. 1 LB (454 g)", MED, 40, kern: 4, cx: cx, top: 2200)
    a.save("\(out)/art_pouch.png")
}

// ---------------- Box front: 2048 x 2731, side: 853 x 2731 ----------------
do {
    let a = Art(2048, 2731)
    let cx: CGFloat = 1024
    a.rect(0, 2270, 2048, 190, ACC)
    a.rect(0, 2500, 2048, 40, ACC)
    a.text("Northwind", DEMI, 270, kern: -3, cx: cx, top: 260)
    a.text("EVERYDAY WELLNESS NATURALLY", MED, 56, kern: 14, cx: cx, top: 590)
    a.rect(300, 780, 1448, 1250, PAPER, radius: 60)
    a.logo(cx, 1160, 250)
    a.text("NOURISHING", DEMI, 110, kern: 14, cx: cx, top: 1520)
    a.text("FACE CREAM", DEMI, 110, kern: 14, cx: cx, top: 1670)
    a.text("HYDRATE  /  PROTECT  /  BALANCE", MED, 48, kern: 10, cx: cx, top: 1870)
    a.text("NET WT. 1.7 OZ (50 g)", MED, 44, kern: 6, cx: cx, top: 2570)
    a.save("\(out)/art_box_front.png")

    let s = Art(853, 2731)
    s.rect(0, 2270, 853, 190, ACC)
    s.rect(0, 2500, 853, 40, ACC)
    s.c.saveGState()
    s.c.translateBy(x: 853 / 2 + 60, y: 2731 / 2 + 100)
    s.c.rotate(by: .pi / 2)
    // Rotated text: draw with a separate small context-free trick: reuse Art.text in rotated space.
    let f = CTFontCreateWithName(DEMI as CFString, 170, nil)
    let line = CTLineCreateWithAttributedString(NSAttributedString(string: "Northwind", attributes: [.font: f, .kern: -2, .foregroundColor: INK]))
    let b = CTLineGetBoundsWithOptions(line, [])
    s.c.textPosition = CGPoint(x: -b.width / 2, y: -50)
    CTLineDraw(line, s.c)
    s.c.restoreGState()
    s.text("50 mL", MED, 44, kern: 6, cx: 853 / 2, top: 2570)
    s.save("\(out)/art_box_side.png")
}

// ---------------- Tote: 2048 x 2363 ----------------
do {
    let a = Art(2048, 2363)
    let cx: CGFloat = 1024
    a.logo(cx, 780, 430)
    a.text("Northwind", DEMI, 300, kern: -4, cx: cx, top: 1330)
    a.text("EVERYDAY WELLNESS NATURALLY", MED, 66, kern: 16, cx: cx, top: 1690)
    a.rect(300, 1880, 1448, 26, ACC)
    a.rect(1480, 2010, 440, 250, PAPER)
    a.text("Northwind", DEMI, 62, kern: 0, cx: 1700, top: 2060)
    a.text("100% ORGANIC COTTON", MED, 26, kern: 4, cx: 1700, top: 2150)
    a.save("\(out)/art_tote.png")
}
print("ok")
