using System.Buffers.Binary;

namespace Md2Hwp.HancomIrPreview;

// Inspect the container and its physical frame without rendering or allocating
// record payloads. Native drawing commands remain Hancom's responsibility.
internal static class EmfDimensions
{
    internal const long MaximumFileBytes = 64L * 1024 * 1024;
    internal const uint MaximumRecords = 1_000_000;

    internal static (long Width, long Height) Read(string path)
    {
        using var stream = File.OpenRead(path);
        InvalidDataException Invalid(string reason) => new($"Invalid or unsupported EMF ({reason}): {path}");
        if (stream.Length is < 108 or > MaximumFileBytes)
            throw Invalid("file must contain a complete header/EOF and be at most 64 MiB");
        var header = new byte[88];
        stream.ReadExactly(header);
        uint U(int offset) => BinaryPrimitives.ReadUInt32LittleEndian(header.AsSpan(offset, 4));
        if (U(0) != 1 || U(40) != 0x464d4520 || U(44) != 0x00010000)
            throw Invalid("missing EMR_HEADER/EMF signature or unsupported version");
        var headerSize = U(4);
        if (headerSize < 88 || headerSize % 4 != 0 || headerSize > stream.Length - 20)
            throw Invalid("invalid header record size");
        if (U(48) != stream.Length)
            throw Invalid("declared byte count does not match the file");
        var expectedRecords = U(52);
        if (expectedRecords is < 2 or > MaximumRecords || expectedRecords > stream.Length / 8)
            throw Invalid("invalid record count or more than 1000000 records");
        var descriptionCharacters = U(60);
        var descriptionOffset = U(64);
        if (descriptionCharacters != 0 && (descriptionOffset < 88 || descriptionOffset % 2 != 0 ||
            (ulong)descriptionOffset + 2UL * descriptionCharacters > headerSize))
            throw Invalid("description exceeds the header record");
        var width = (long)BinaryPrimitives.ReadInt32LittleEndian(header.AsSpan(32)) - BinaryPrimitives.ReadInt32LittleEndian(header.AsSpan(24));
        var height = (long)BinaryPrimitives.ReadInt32LittleEndian(header.AsSpan(36)) - BinaryPrimitives.ReadInt32LittleEndian(header.AsSpan(28));
        if (width <= 0 || height <= 0)
            throw Invalid("physical frame must have positive width and height");

        stream.Position = headerSize;
        uint count = 1;
        Span<byte> record = stackalloc byte[8];
        Span<byte> eof = stackalloc byte[12];
        while (stream.Position < stream.Length)
        {
            var start = stream.Position;
            if (stream.Length - start < 8) throw Invalid("truncated record header");
            stream.ReadExactly(record);
            var type = BinaryPrimitives.ReadUInt32LittleEndian(record);
            var size = BinaryPrimitives.ReadUInt32LittleEndian(record[4..]);
            if (type is 0 or 1 || size < 8 || size % 4 != 0 || size > stream.Length - start)
                throw Invalid("invalid record type, size or boundary");
            if (++count > expectedRecords) throw Invalid("too many records");
            if (type == 14)
            {
                if (size < 20 || start + size != stream.Length || count != expectedRecords)
                    throw Invalid("invalid EOF position, size or record count");
                stream.ReadExactly(eof);
                var entries = BinaryPrimitives.ReadUInt32LittleEndian(eof);
                var offset = BinaryPrimitives.ReadUInt32LittleEndian(eof[4..]);
                if (BinaryPrimitives.ReadUInt32LittleEndian(eof[8..]) != size ||
                    (entries != 0 && (offset < 20 || (ulong)offset + 4UL * entries > size)))
                    throw Invalid("invalid EOF palette or size");
                return (width, height);
            }
            stream.Position = start + size;
        }
        throw Invalid("missing EMR_EOF");
    }
}
