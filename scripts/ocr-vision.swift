#!/usr/bin/env swift
import Foundation
import Vision
import CoreGraphics
import ImageIO
import UniformTypeIdentifiers

struct Line: Codable {
    let text: String
    let confidence: Float
    let bbox: [Double] // normalized [x, y, width, height]
}

struct PageOutput: Codable {
    let page: Int
    let lines: [Line]
}

func parsePages(_ pages: String?, total: Int) -> [Int] {
    guard let pages = pages, !pages.isEmpty else {
        return Array(1...total)
    }

    var result = Set<Int>()
    for part in pages.split(separator: ",") {
        let trimmed = part.trimmingCharacters(in: .whitespacesAndNewlines)
        if trimmed.isEmpty { continue }
        if let dash = trimmed.firstIndex(of: "-") {
            let startStr = String(trimmed[..<dash])
            let endStr = String(trimmed[trimmed.index(after: dash)...])
            if let start = Int(startStr), let end = Int(endStr) {
                let lo = min(start, end)
                let hi = max(start, end)
                for page in lo...hi where page >= 1 && page <= total {
                    result.insert(page)
                }
            }
        } else if let page = Int(trimmed), page >= 1 && page <= total {
            result.insert(page)
        }
    }

    return result.sorted()
}

func renderPage(_ page: CGPDFPage, dpi: Double) -> CGImage {
    let rect = page.getBoxRect(.mediaBox)
    let scale = dpi / 72.0
    let width = Int(rect.width * CGFloat(scale))
    let height = Int(rect.height * CGFloat(scale))

    let colorSpace = CGColorSpaceCreateDeviceRGB()
    let bitmapInfo = CGImageAlphaInfo.premultipliedLast.rawValue
    guard let context = CGContext(
        data: nil,
        width: width,
        height: height,
        bitsPerComponent: 8,
        bytesPerRow: 0,
        space: colorSpace,
        bitmapInfo: bitmapInfo
    ) else {
        fatalError("Failed to create bitmap context")
    }

    context.interpolationQuality = .high
    context.setShouldAntialias(true)
    context.setAllowsAntialiasing(true)

    context.setFillColor(CGColor(red: 1, green: 1, blue: 1, alpha: 1))
    context.fill(CGRect(x: 0, y: 0, width: width, height: height))

    let targetRect = CGRect(x: 0, y: 0, width: CGFloat(width), height: CGFloat(height))
    let transform = page.getDrawingTransform(.mediaBox, rect: targetRect, rotate: 0, preserveAspectRatio: true)
    context.concatenate(transform)
    context.drawPDFPage(page)

    guard let image = context.makeImage() else {
        fatalError("Failed to create CGImage")
    }
    return image
}

func saveImage(_ image: CGImage, to url: URL) throws {
    let dir = url.deletingLastPathComponent()
    if !dir.path.isEmpty {
        try FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
    }
    guard let dest = CGImageDestinationCreateWithURL(url as CFURL, UTType.png.identifier as CFString, 1, nil) else {
        throw NSError(domain: "ocr-vision", code: 1, userInfo: [NSLocalizedDescriptionKey: "Failed to create image destination"])
    }
    CGImageDestinationAddImage(dest, image, nil)
    if !CGImageDestinationFinalize(dest) {
        throw NSError(domain: "ocr-vision", code: 2, userInfo: [NSLocalizedDescriptionKey: "Failed to write image"])
    }
}

func loadImage(from url: URL) throws -> CGImage {
    guard let source = CGImageSourceCreateWithURL(url as CFURL, nil) else {
        throw NSError(domain: "ocr-vision", code: 3, userInfo: [NSLocalizedDescriptionKey: "Failed to open image source"])
    }
    guard let image = CGImageSourceCreateImageAtIndex(source, 0, nil) else {
        throw NSError(domain: "ocr-vision", code: 4, userInfo: [NSLocalizedDescriptionKey: "Failed to decode image"])
    }
    return image
}

func ocrPage(
    _ image: CGImage,
    candidateCount: Int,
    recognitionLevel: VNRequestTextRecognitionLevel,
    usesLanguageCorrection: Bool,
    minTextHeight: Float?
) throws -> [Line] {
    let request = VNRecognizeTextRequest()
    request.recognitionLevel = recognitionLevel
    request.usesLanguageCorrection = usesLanguageCorrection
    request.recognitionLanguages = ["nl-NL", "en-US"]
    if let minTextHeight = minTextHeight {
        request.minimumTextHeight = minTextHeight
    }

    let handler = VNImageRequestHandler(cgImage: image, options: [:])
    try handler.perform([request])

    let observations = request.results ?? []

    var lines: [Line] = []
    let count = max(1, candidateCount)
    for obs in observations {
        let candidates = obs.topCandidates(count)
        if candidates.isEmpty { continue }
        let bbox = obs.boundingBox
        for candidate in candidates {
            lines.append(Line(
                text: candidate.string,
                confidence: candidate.confidence,
                bbox: [Double(bbox.origin.x), Double(bbox.origin.y), Double(bbox.size.width), Double(bbox.size.height)]
            ))
        }
    }
    return lines
}

func usage() {
    let msg = """
    Usage:
            scripts/ocr-vision.swift <pdf> [--dpi 300] [--pages 1,3-5] [--out output.json] [--image-out page.png] [--image-dir dir]
                                [--candidates 1] [--level accurate|fast] [--no-language-correction] [--min-text-height 0.02]
            scripts/ocr-vision.swift --image input.png [--out output.json] [--image-out page.png]
                                [--candidates 1] [--level accurate|fast] [--no-language-correction] [--min-text-height 0.02]
            scripts/ocr-vision.swift --image-dir-input dir [--out output.json]
                                [--candidates 1] [--level accurate|fast] [--no-language-correction] [--min-text-height 0.02]
    """
    print(msg)
}

let args = CommandLine.arguments
if args.count < 2 {
    usage()
    exit(1)
}

var pdfPath: String? = nil
var dpi: Double = 300
var pagesArg: String? = nil
var outPath: String? = nil
var imageOut: String? = nil
var imageDir: String? = nil
var imagePath: String? = nil
var imageDirInput: String? = nil
var candidates: Int = 1
var level: String = "accurate"
var usesLanguageCorrection: Bool = true
var minTextHeight: Float? = nil

var i = 1
while i < args.count {
    let arg = args[i]
    switch arg {
    case "--dpi":
        i += 1
        if i < args.count, let value = Double(args[i]) { dpi = value }
    case "--pages":
        i += 1
        if i < args.count { pagesArg = args[i] }
    case "--out":
        i += 1
        if i < args.count { outPath = args[i] }
    case "--image-out":
        i += 1
        if i < args.count { imageOut = args[i] }
    case "--image-dir":
        i += 1
        if i < args.count { imageDir = args[i] }
    case "--image":
        i += 1
        if i < args.count { imagePath = args[i] }
    case "--image-dir-input":
        i += 1
        if i < args.count { imageDirInput = args[i] }
    case "--candidates":
        i += 1
        if i < args.count, let value = Int(args[i]) { candidates = value }
    case "--level":
        i += 1
        if i < args.count { level = args[i].lowercased() }
    case "--no-language-correction":
        usesLanguageCorrection = false
    case "--min-text-height":
        i += 1
        if i < args.count, let value = Float(args[i]) { minTextHeight = value }
    default:
        if pdfPath == nil { pdfPath = arg }
    }
    i += 1
}

let recognitionLevel: VNRequestTextRecognitionLevel = (level == "fast") ? .fast : .accurate

if let imagePath = imagePath {
    if pdfPath != nil {
        fatalError("Provide either a PDF path or --image, not both")
    }
    if imageDirInput != nil {
        fatalError("Provide either --image or --image-dir-input, not both")
    }
    let imageURL = URL(fileURLWithPath: imagePath)
    if !FileManager.default.fileExists(atPath: imagePath) {
        fatalError("Image not found: \(imagePath)")
    }
    let image = try loadImage(from: imageURL)
    if let imageOut = imageOut {
        try saveImage(image, to: URL(fileURLWithPath: imageOut))
    }
    let lines = try ocrPage(
        image,
        candidateCount: candidates,
        recognitionLevel: recognitionLevel,
        usesLanguageCorrection: usesLanguageCorrection,
        minTextHeight: minTextHeight
    )
    let output = [PageOutput(page: 1, lines: lines)]
    let encoder = JSONEncoder()
    encoder.outputFormatting = [.prettyPrinted, .sortedKeys]
    let data = try encoder.encode(output)

    if let outPath = outPath {
        let outURL = URL(fileURLWithPath: outPath)
        if !outURL.deletingLastPathComponent().path.isEmpty {
            try FileManager.default.createDirectory(at: outURL.deletingLastPathComponent(), withIntermediateDirectories: true)
        }
        try data.write(to: outURL)
    } else {
        if let text = String(data: data, encoding: .utf8) {
            print(text)
        }
    }
    exit(0)
}

if let imageDirInput = imageDirInput {
    if pdfPath != nil {
        fatalError("Provide either a PDF path or --image-dir-input, not both")
    }
    let dirURL = URL(fileURLWithPath: imageDirInput)
    let fm = FileManager.default
    guard let entries = try? fm.contentsOfDirectory(at: dirURL, includingPropertiesForKeys: nil) else {
        fatalError("Failed to read directory: \(imageDirInput)")
    }
    let images = entries
        .filter { ["jpg", "jpeg", "png"].contains($0.pathExtension.lowercased()) }
        .sorted { $0.lastPathComponent < $1.lastPathComponent }

    if images.isEmpty {
        fatalError("No images found in directory: \(imageDirInput)")
    }

    var output: [PageOutput] = []
    for (idx, url) in images.enumerated() {
        let image = try loadImage(from: url)
        let lines = try ocrPage(
            image,
            candidateCount: candidates,
            recognitionLevel: recognitionLevel,
            usesLanguageCorrection: usesLanguageCorrection,
            minTextHeight: minTextHeight
        )
        output.append(PageOutput(page: idx + 1, lines: lines))
    }

    let encoder = JSONEncoder()
    encoder.outputFormatting = [.prettyPrinted, .sortedKeys]
    let data = try encoder.encode(output)

    if let outPath = outPath {
        let outURL = URL(fileURLWithPath: outPath)
        if !outURL.deletingLastPathComponent().path.isEmpty {
            try FileManager.default.createDirectory(at: outURL.deletingLastPathComponent(), withIntermediateDirectories: true)
        }
        try data.write(to: outURL)
    } else {
        if let text = String(data: data, encoding: .utf8) {
            print(text)
        }
    }
    exit(0)
}

guard let pdfPathUnwrapped = pdfPath else {
    usage()
    exit(1)
}

let pdfURL = URL(fileURLWithPath: pdfPathUnwrapped)
if !FileManager.default.fileExists(atPath: pdfPathUnwrapped) {
    fatalError("PDF not found: \(pdfPathUnwrapped)")
}

guard let doc = CGPDFDocument(pdfURL as CFURL) else {
    fatalError("Failed to open PDF")
}

let totalPages = doc.numberOfPages
if totalPages <= 0 {
    fatalError("PDF has no pages")
}

let pages = parsePages(pagesArg, total: totalPages)
if pages.isEmpty {
    fatalError("No pages selected")
}

var output: [PageOutput] = []
for pageIndex in pages {
    guard let page = doc.page(at: pageIndex) else { continue }
    let image = renderPage(page, dpi: dpi)
    if let imageOut = imageOut, pages.count == 1 {
        try saveImage(image, to: URL(fileURLWithPath: imageOut))
    }
    if let imageDir = imageDir {
        let filename = String(format: "page-%03d.png", pageIndex)
        let url = URL(fileURLWithPath: imageDir).appendingPathComponent(filename)
        try saveImage(image, to: url)
    }
    let lines = try ocrPage(
        image,
        candidateCount: candidates,
        recognitionLevel: recognitionLevel,
        usesLanguageCorrection: usesLanguageCorrection,
        minTextHeight: minTextHeight
    )
    output.append(PageOutput(page: pageIndex, lines: lines))
}

let encoder = JSONEncoder()
encoder.outputFormatting = [.prettyPrinted, .sortedKeys]
let data = try encoder.encode(output)

if let outPath = outPath {
    let outURL = URL(fileURLWithPath: outPath)
    if !outURL.deletingLastPathComponent().path.isEmpty {
        try FileManager.default.createDirectory(at: outURL.deletingLastPathComponent(), withIntermediateDirectories: true)
    }
    try data.write(to: outURL)
} else {
    if let text = String(data: data, encoding: .utf8) {
        print(text)
    }
}
