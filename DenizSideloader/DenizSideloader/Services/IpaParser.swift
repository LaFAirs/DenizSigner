import Compression
import Foundation

/// Best-effort IPA metadata reader (no third-party code).
///
/// Parses the ZIP central directory, extracts `Payload/*.app/Info.plist`
/// (stored or deflate-compressed) and returns bundle ID, version and
/// display name. Anything it cannot prove is reported as an error —
/// callers keep the file and let the user confirm the details manually.
///
/// Limits (documented, not hidden): no Zip64, single best-match
/// Info.plist, 200 MiB inflation cap.
struct IpaMetadata {
    var displayName: String
    var bundleID: String
    var version: String
}

enum IpaParserError: LocalizedError {
    case notZip
    case noAppBundle
    case plistUnreadable
    case tooLarge
    case unsupported(String)

    var errorDescription: String? {
        switch self {
        case .notZip: return NSLocalizedString("ipa.error.notzip", comment: "")
        case .noAppBundle: return NSLocalizedString("ipa.error.noapp", comment: "")
        case .plistUnreadable: return NSLocalizedString("ipa.error.plist", comment: "")
        case .tooLarge: return NSLocalizedString("ipa.error.toolarge", comment: "")
        case let .unsupported(detail): return detail
        }
    }
}

private struct ZipReader {
    let data: Data
    func u16(at offset: Int) -> UInt16? {
        guard offset + 2 <= data.count else { return nil }
        return data.withUnsafeBytes { UInt16(littleEndian: $0.loadUnaligned(fromByteOffset: offset, as: UInt16.self)) }
    }
    func u32(at offset: Int) -> UInt32? {
        guard offset + 4 <= data.count else { return nil }
        return data.withUnsafeBytes { UInt32(littleEndian: $0.loadUnaligned(fromByteOffset: offset, as: UInt32.self)) }
    }
    func bytes(at offset: Int, length: Int) -> Data? {
        guard length >= 0, offset >= 0, offset + length <= data.count else { return nil }
        return data.subdata(in: offset ..< offset + length)
    }
}

enum IpaParser {
    static let maxInflateBytes = 200_000_000

    static func parse(ipaURL: URL) throws -> IpaMetadata {
        let data = try Data(contentsOf: ipaURL, options: .mappedIfSafe)
        let reader = ZipReader(data: data)
        guard let eocd = findEOCD(in: reader) else { throw IpaParserError.notZip }
        let plistData = try extractInfoPlist(reader: reader, eocd: eocd)
        return try readMetadata(from: plistData)
    }

    // MARK: - ZIP

    private struct EOCD { var centralOffset: Int; var centralCount: Int }

    private static func findEOCD(in r: ZipReader) -> EOCD? {
        // EOCD is at most 22 + 65535 bytes from the end.
        let tail = min(r.data.count, 22 + 65_535)
        var i = r.data.count - 22
        let stop = r.data.count - tail
        while i >= stop {
            if r.u32(at: i) == 0x0605_4B50,
               let count = r.u16(at: i + 10),
               let offset = r.u32(at: i + 16),
               count > 0, offset != 0xFFFF_FFFF
            {
                return EOCD(centralOffset: Int(offset), centralCount: Int(count))
            }
            i -= 1
        }
        return nil
    }

    private static func extractInfoPlist(reader r: ZipReader, eocd: EOCD) throws -> Data {
        var cursor = eocd.centralOffset
        var best: (localOffset: Int, compSize: Int, method: Int, name: String)?
        for _ in 0 ..< eocd.centralCount {
            guard r.u32(at: cursor) == 0x0201_4B50,
                  let method = r.u16(at: cursor + 10),
                  let compSize = r.u32(at: cursor + 20),
                  let uncompSize = r.u32(at: cursor + 24),
                  let nameLen = r.u16(at: cursor + 28),
                  let extraLen = r.u16(at: cursor + 30),
                  let commentLen = r.u16(at: cursor + 32),
                  let localOffset = r.u32(at: cursor + 42),
                  let nameData = r.bytes(at: cursor + 46, length: Int(nameLen)),
                  let name = String(data: nameData, encoding: .utf8)
            else { break }
            if compSize == 0xFFFF_FFFF || uncompSize == 0xFFFF_FFFF || localOffset == 0xFFFF_FFFF {
                throw IpaParserError.unsupported(NSLocalizedString("ipa.error.zip64", comment: ""))
            }
            if name.hasPrefix("Payload/"), name.hasSuffix(".app/Info.plist") {
                // Prefer the shortest path (top-level app bundle).
                if best == nil || name.count < best!.name.count {
                    best = (Int(localOffset), Int(compSize), Int(method), name)
                }
            }
            cursor += 46 + Int(nameLen) + Int(extraLen) + Int(commentLen)
        }
        guard let hit = best else { throw IpaParserError.noAppBundle }
        return try readLocalFile(reader: r, at: hit.localOffset, compSize: hit.compSize, method: hit.method)
    }

    private static func readLocalFile(reader r: ZipReader, at offset: Int, compSize: Int, method: Int) throws -> Data {
        guard r.u32(at: offset) == 0x0403_4B50,
              let nameLen = r.u16(at: offset + 26),
              let extraLen = r.u16(at: offset + 28)
        else { throw IpaParserError.notZip }
        let dataStart = offset + 30 + Int(nameLen) + Int(extraLen)
        guard let raw = r.bytes(at: dataStart, length: compSize) else { throw IpaParserError.notZip }
        switch method {
        case 0:
            return raw
        case 8:
            guard let inflated = inflateZlib(raw) else { throw IpaParserError.plistUnreadable }
            return inflated
        default:
            throw IpaParserError.unsupported(NSLocalizedString("ipa.error.method", comment: ""))
        }
    }

    private static func inflateZlib(_ data: Data) -> Data? {
        // Decodes raw deflate content with a bounded output cap. All
        // pointers stay inside their `withUnsafe...` scopes.
        var output = Data()
        let chunk = 1_048_576
        var buffer = [UInt8](repeating: 0, count: chunk)
        let status: compression_status = buffer.withUnsafeMutableBufferPointer { buf in
            data.withUnsafeBytes { input -> compression_status in
                guard let inBase = input.baseAddress?.assumingMemoryBound(to: UInt8.self),
                      let outBase = buf.baseAddress
                else {
                    return COMPRESSION_STATUS_ERROR
                }
                var stream = compression_stream(
                    dst_ptr: outBase, dst_size: 0,
                    src_ptr: inBase, src_size: data.count, state: nil)
                guard compression_stream_init(&stream, COMPRESSION_STREAM_DECODE, COMPRESSION_ZLIB)
                    == COMPRESSION_STATUS_OK
                else {
                    return COMPRESSION_STATUS_ERROR
                }
                defer { compression_stream_destroy(&stream) }
                var st: compression_status = COMPRESSION_STATUS_OK
                while st == COMPRESSION_STATUS_OK {
                    stream.dst_ptr = outBase
                    stream.dst_size = chunk
                    st = compression_stream_process(&stream, 0)
                    let produced = chunk - stream.dst_size
                    if produced > 0 {
                        if output.count + produced > maxInflateBytes {
                            return COMPRESSION_STATUS_ERROR
                        }
                        output.append(contentsOf: buf.prefix(produced))
                    }
                }
                return st == COMPRESSION_STATUS_END ? COMPRESSION_STATUS_END : COMPRESSION_STATUS_ERROR
            }
        }
        return status == COMPRESSION_STATUS_END ? output : nil
    }

    // MARK: - Plist

    private static func readMetadata(from plistData: Data) throws -> IpaMetadata {
        guard let plist = try? PropertyListSerialization.propertyList(from: plistData, format: nil),
              let dict = plist as? [String: Any]
        else { throw IpaParserError.plistUnreadable }
        guard let bundleID = dict["CFBundleIdentifier"] as? String, !bundleID.isEmpty else {
            throw IpaParserError.plistUnreadable
        }
        let version = (dict["CFBundleShortVersionString"] as? String)
            ?? (dict["CFBundleVersion"] as? String) ?? "?"
        let name = (dict["CFBundleDisplayName"] as? String)
            ?? (dict["CFBundleName"] as? String) ?? bundleID
        return IpaMetadata(displayName: name, bundleID: bundleID, version: version)
    }
}
