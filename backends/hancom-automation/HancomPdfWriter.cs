using System.Diagnostics;
using System.Globalization;
using System.Text;
using System.Text.RegularExpressions;

namespace Md2Hwp.HancomIrPreview;

internal sealed record PdfExportOptions(string SourcePath, string OutputPath)
{
    internal const string Usage = """
        usage: hwp2pdf <source.hwp> [[--output] <source.pdf>]
               hwp2pdf --input <source.hwp> [--output <source.pdf>]
               hwp2pdf --version
        Default output: source.pdf beside the input HWP.
        Requires Windows x64, Hancom HWP, .NET 10 x64 and the registered Hancom security module.
        """;

    internal static PdfExportOptions Parse(string[] args)
    {
        string? source = null, output = null;
        var seen = new HashSet<string>(StringComparer.Ordinal);
        for (var index = 0; index < args.Length; index++)
        {
            var value = args[index];
            if (value is "--input" or "--output")
            {
                if (!seen.Add(value)) throw new ArgumentException($"Duplicate argument: {value}");
                if (++index >= args.Length || args[index].StartsWith("-", StringComparison.Ordinal))
                    throw new ArgumentException($"Missing value for {value}.\n{Usage}");
                if (value == "--input")
                {
                    if (source is not null) throw new ArgumentException("Duplicate input argument.");
                    source = args[index];
                }
                else
                {
                    if (output is not null) throw new ArgumentException("Duplicate output argument.");
                    output = args[index];
                }
            }
            else
            {
                if (value.StartsWith("-", StringComparison.Ordinal)) throw new ArgumentException(Usage);
                if (source is null) source = value;
                else if (output is null) output = value;
                else throw new ArgumentException(Usage);
            }
        }
        if (string.IsNullOrWhiteSpace(source)) throw new ArgumentException(Usage);
        source = Path.GetFullPath(source);
        output = Path.GetFullPath(output ?? Path.ChangeExtension(source, ".pdf"));
        if (!Path.GetExtension(source).Equals(".hwp", StringComparison.OrdinalIgnoreCase))
            throw new ArgumentException("Input must be .hwp; HWPX input is not supported.");
        if (!Path.GetExtension(output).Equals(".pdf", StringComparison.OrdinalIgnoreCase))
            throw new ArgumentException("Output must be .pdf.");
        return new(source, output);
    }

    internal void ValidatePaths()
    {
        if (!File.Exists(SourcePath)) throw new FileNotFoundException($"Missing HWP: {SourcePath}", SourcePath);
        if (!Directory.Exists(Path.GetDirectoryName(OutputPath)))
            throw new DirectoryNotFoundException($"Missing output directory: {Path.GetDirectoryName(OutputPath)}");
        if (Directory.Exists(OutputPath)) throw new IOException($"Output is a directory: {OutputPath}");
        var source = new FileInfo(SourcePath).ResolveLinkTarget(true)?.FullName ?? SourcePath;
        var output = File.Exists(OutputPath)
            ? new FileInfo(OutputPath).ResolveLinkTarget(true)?.FullName ?? OutputPath : OutputPath;
        if (source.Equals(output, StringComparison.OrdinalIgnoreCase))
            throw new ArgumentException("Output must not replace the input HWP.");
    }
}

internal sealed record PdfExportResult(string Output, int SourcePages, long Bytes, int GraphicQuality);

internal static partial class HancomPreviewWriter
{
    internal static PdfExportResult ExportPdf(PdfExportOptions options)
    {
        options.ValidatePaths();
        EnsureInteractiveContext();
        EnsureNoExistingHwpProcess();
        // Some Hancom installations use the Hwp64 process name.
        foreach (var process in Process.GetProcessesByName("Hwp64"))
        {
            using (process) throw new InvalidOperationException("Close existing HWP processes before PDF conversion.");
        }
        var module = SecurityModuleRegistration.ReadAndValidate(AppContext.BaseDirectory);
        var sourceHash = HashFile(options.SourcePath);
        var temporaryDirectory = Path.Combine(Path.GetDirectoryName(options.OutputPath)!, ".hwp2pdf-" + Guid.NewGuid().ToString("N"));
        Directory.CreateDirectory(temporaryDirectory);
        var temporaryPdf = Path.Combine(temporaryDirectory, "output.pdf");
        try
        {
            var pages = WithHwp(module, hwp =>
            {
                hwp.XHwpWindows.Item(0).Visible = false;
                Open(hwp, options.SourcePath, visible: false);
                var pageCount = (int)hwp.PageCount;
                if (pageCount < 1) throw new InvalidOperationException("HWP has no printable pages.");
                dynamic action = hwp.CreateAction("PrintToPDFEx");
                dynamic set = action.CreateSet();
                if (Convert.ToInt32(action.GetDefault(set), CultureInfo.InvariantCulture) == 0)
                    throw new InvalidOperationException("PrintToPDFEx.GetDefault failed.");
                foreach (var (name, value) in PdfExportFile.Parameters(temporaryPdf)) set.SetItem(name, value);
                foreach (var (name, value) in PdfExportFile.Parameters(temporaryPdf))
                {
                    object? actual = set.Item(name);
                    if (actual is null || (value is string text
                            ? !string.Equals(actual.ToString(), text, StringComparison.Ordinal)
                            : Convert.ToInt32(actual, CultureInfo.InvariantCulture) != (int)value))
                        throw new InvalidOperationException($"PrintToPDFEx did not accept {name}={value}.");
                }
                // CreateAction/CreateSet must execute through the same Action COM interface.
                if (!(bool)action.Execute(set)) throw new InvalidOperationException("PrintToPDFEx failed.");
                PdfExportFile.WaitForComplete(temporaryPdf, TimeSpan.FromMinutes(5));
                return pageCount;
            });
            if (!HashFile(options.SourcePath).Equals(sourceHash, StringComparison.Ordinal))
                throw new InvalidOperationException("Source HWP changed during PDF conversion; output was not replaced.");
            var bytes = PdfExportFile.Publish(temporaryPdf, options.OutputPath);
            return new(options.OutputPath, pages, bytes, PdfExportFile.GraphicQuality);
        }
        finally
        {
            if (File.Exists(temporaryPdf)) File.Delete(temporaryPdf);
            Directory.Delete(temporaryDirectory, recursive: false);
        }
    }
}

internal static class PdfExportFile
{
    internal const int GraphicQuality = 100;

    internal static IReadOnlyDictionary<string, object> Parameters(string output) => new Dictionary<string, object>
    {
        ["FileName"] = output, ["Device"] = 5, ["Range"] = 6,
        ["PrintMethod"] = 0, ["ZoomX"] = 100, ["ZoomY"] = 100,
        ["PrintImage"] = 1, ["PrintDrawObj"] = 1,
        ["PrintAutoHeadNote"] = 0, ["PrintAutoFootNote"] = 0,
        ["NumCopy"] = 1, ["ReverseOrder"] = 0, ["GraphicQuality"] = GraphicQuality,
    };

    internal static void WaitForComplete(string path, TimeSpan timeout)
    {
        var watch = Stopwatch.StartNew();
        long previousLength = -1;
        var stable = 0;
        Exception? lastError = null;
        while (watch.Elapsed < timeout)
        {
            try
            {
                var length = ValidateComplete(path);
                stable = length == previousLength ? stable + 1 : 0;
                previousLength = length;
                if (stable >= 2) return;
            }
            catch (IOException error) { lastError = error; stable = 0; }
            catch (InvalidDataException error) { lastError = error; stable = 0; }
            Thread.Sleep(200);
        }
        throw new TimeoutException("PDF output did not finish within the wait period.", lastError);
    }

    // Completion/envelope check, not a full PDF object or visual-layout validator.
    internal static long ValidateComplete(string path)
    {
        using var stream = new FileStream(path, FileMode.Open, FileAccess.Read, FileShare.None);
        if (stream.Length < 20) throw new InvalidDataException("Incomplete PDF output.");
        Span<byte> header = stackalloc byte[5];
        stream.ReadExactly(header);
        if (!header.SequenceEqual("%PDF-"u8)) throw new InvalidDataException("Output is not a PDF.");
        var tailLength = (int)Math.Min(stream.Length, 4096);
        stream.Position = stream.Length - tailLength;
        var tail = new byte[tailLength];
        stream.ReadExactly(tail);
        var trailer = Encoding.ASCII.GetString(tail);
        var match = Regex.Match(trailer, @"startxref\s+([0-9]+)\s+%%EOF[\x00\t\n\f\r ]*\z", RegexOptions.CultureInvariant,
            TimeSpan.FromSeconds(1));
        if (!match.Success || !long.TryParse(match.Groups[1].Value, NumberStyles.None, CultureInfo.InvariantCulture, out var offset)
            || offset < 5 || offset >= stream.Length - tailLength + match.Index)
            throw new InvalidDataException("PDF is missing its final cross-reference trailer.");
        stream.Position = offset;
        var xref = new byte[(int)Math.Min(128, stream.Length - offset)];
        stream.ReadExactly(xref);
        var start = Encoding.ASCII.GetString(xref);
        if (!start.StartsWith("xref", StringComparison.Ordinal) &&
            !Regex.IsMatch(start, @"\A[0-9]+\s+[0-9]+\s+obj\b", RegexOptions.CultureInvariant, TimeSpan.FromSeconds(1)))
            throw new InvalidDataException("PDF cross-reference offset is invalid.");
        return stream.Length;
    }

    internal static long Publish(string temporaryPdf, string output)
    {
        var bytes = ValidateComplete(temporaryPdf);
        File.Move(temporaryPdf, output, overwrite: true);
        return bytes;
    }
}
