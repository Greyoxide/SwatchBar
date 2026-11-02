// Usage: swift make_icon.swift raw.png out.iconset
// Removes the white backdrop (flood fill from the edges), fits the squircle to the
// macOS icon grid (824/1024) and writes every iconset size.
import Foundation
import CoreGraphics
import ImageIO
import UniformTypeIdentifiers

let args = CommandLine.arguments
let src = CGImageSourceCreateWithURL(URL(fileURLWithPath: args[1]) as CFURL, nil)!
let img = CGImageSourceCreateImageAtIndex(src, 0, nil)!
let w = img.width, h = img.height
var px = [UInt8](repeating: 0, count: w * h * 4)
let cs = CGColorSpace(name: CGColorSpace.sRGB)!
let ctx = CGContext(data: &px, width: w, height: h, bitsPerComponent: 8, bytesPerRow: w * 4,
                    space: cs, bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue)!
ctx.draw(img, in: CGRect(x: 0, y: 0, width: w, height: h))

func whiteness(_ i: Int) -> Int { Int(min(px[i], px[i + 1], px[i + 2])) }
var bg = [Bool](repeating: false, count: w * h)
var stack: [Int] = []
func seed(_ x: Int, _ y: Int) {
    let p = y * w + x
    if !bg[p] && whiteness(p * 4) > 215 { bg[p] = true; stack.append(p) }
}
for x in 0..<w { seed(x, 0); seed(x, h - 1) }
for y in 0..<h { seed(0, y); seed(w - 1, y) }
while let p = stack.popLast() {
    let x = p % w, y = p / w
    if x > 0 { seed(x - 1, y) }; if x < w - 1 { seed(x + 1, y) }
    if y > 0 { seed(x, y - 1) }; if y < h - 1 { seed(x, y + 1) }
}
// Clear background; feather the fringe next to it so no white halo remains.
var out = px
for y in 0..<h { for x in 0..<w {
    let p = y * w + x
    if bg[p] { out[p * 4] = 0; out[p * 4 + 1] = 0; out[p * 4 + 2] = 0; out[p * 4 + 3] = 0; continue }
    var near = false
    for (dx, dy) in [(-1, 0), (1, 0), (0, -1), (0, 1), (-2, 0), (2, 0), (0, -2), (0, 2)] {
        let nx = x + dx, ny = y + dy
        if nx >= 0, ny >= 0, nx < w, ny < h, bg[ny * w + nx] { near = true; break }
    }
    if near {
        // Fade by how white it is; un-premultiply toward the dark squircle colour.
        let wh = Double(whiteness(p * 4))
        let a = max(0.0, min(1.0, (255.0 - wh) / 150.0))
        out[p * 4 + 3] = UInt8(a * 255)
        for c in 0..<3 { out[p * 4 + c] = UInt8(Double(out[p * 4 + c]) * a) }
    }
}}
let cut = CGContext(data: &out, width: w, height: h, bitsPerComponent: 8, bytesPerRow: w * 4,
                    space: cs, bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue)!.makeImage()!

// Tight bounding box of the squircle, then fit to 824/1024 of each target size.
var minX = w, maxX = 0, minY = h, maxY = 0
for y in 0..<h { for x in 0..<w where out[(y * w + x) * 4 + 3] > 128 {
    minX = min(minX, x); maxX = max(maxX, x); minY = min(minY, y); maxY = max(maxY, y)
}}
let crop = cut.cropping(to: CGRect(x: minX, y: minY, width: maxX - minX + 1, height: maxY - minY + 1))!

func write(_ size: Int, _ name: String) {
    let c = CGContext(data: nil, width: size, height: size, bitsPerComponent: 8, bytesPerRow: 0,
                      space: cs, bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue)!
    c.interpolationQuality = .high
    let side = Double(size) * 824.0 / 1024.0
    let o = (Double(size) - side) / 2
    c.draw(crop, in: CGRect(x: o, y: o, width: side, height: side))
    let dest = CGImageDestinationCreateWithURL(URL(fileURLWithPath: "\(args[2])/\(name)") as CFURL,
                                               UTType.png.identifier as CFString, 1, nil)!
    CGImageDestinationAddImage(dest, c.makeImage()!, nil)
    CGImageDestinationFinalize(dest)
}
try? FileManager.default.createDirectory(atPath: args[2], withIntermediateDirectories: true)
for s in [16, 32, 128, 256, 512] {
    write(s, "icon_\(s)x\(s).png")
    write(s * 2, "icon_\(s)x\(s)@2x.png")
}
