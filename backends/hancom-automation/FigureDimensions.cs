namespace Md2Hwp.HancomIrPreview;

internal static class FigureDimensions
{
    internal static (long Width, long Height) Read(string path) => Path.GetExtension(path).ToLowerInvariant() switch
    {
        ".png" => PngDimensions.Read(path),
        ".jpg" or ".jpeg" => ReadJpeg(path),
        ".emf" => EmfDimensions.Read(path),
        _ => throw new InvalidDataException("Figures support PNG, JPG, JPEG and EMF files only."),
    };

    private static (int Width, int Height) ReadJpeg(string path)
    {
        using var stream = File.OpenRead(path);
        InvalidDataException Invalid(string message) => new($"Invalid or unsupported JPEG ({message}): {path}");
        int Byte() { var value = stream.ReadByte(); return value >= 0 ? value : throw Invalid("truncated header"); }
        int Word() => (Byte() << 8) | Byte();
        if (Word() != 0xffd8) throw Invalid("missing SOI signature");
        while (stream.Position < stream.Length)
        {
            if (Byte() != 0xff) throw Invalid("missing segment marker");
            int marker;
            do { marker = Byte(); } while (marker == 0xff);
            if (marker is 0 or 0xd8 or >= 0xd0 and <= 0xd7) throw Invalid("unexpected marker before frame");
            if (marker is 0xda or 0xd9) throw Invalid("missing frame dimensions");
            if (marker == 0x01) continue; // Standalone TEM marker has no segment length.
            var length = Word();
            if (length < 2 || length - 2 > stream.Length - stream.Position) throw Invalid("invalid segment length");
            if (marker is 0xc0 or 0xc1 or 0xc2)
            {
                if (length < 8) throw Invalid("truncated frame header");
                var precision = Byte();
                var height = Word();
                var width = Word();
                var components = Byte();
                if (precision != 8 || width == 0 || height == 0 || components is < 1 or > 4 || length != 8 + 3 * components)
                    throw Invalid("invalid or unsupported frame dimensions/precision/components");
                return (width, height);
            }
            if (marker is >= 0xc0 and <= 0xcf && marker is not (0xc4 or 0xc8 or 0xcc))
                throw Invalid("only 8-bit sequential and progressive DCT frames are supported");
            stream.Seek(length - 2, SeekOrigin.Current);
        }
        throw Invalid("missing frame header");
    }
}
