using System.Text;
using Md2Hwp.HancomIrPreview;

internal static class PdfExportTests
{
    internal static void Run()
    {
        static void Check(bool condition, string message) { if (!condition) throw new Exception(message); }
        static void Reject(Action action)
        {
            try { action(); }
            catch (Exception error) when (error is ArgumentException or IOException or InvalidDataException or UnauthorizedAccessException) { return; }
            throw new Exception("Expected PDF export rejection.");
        }
        var options = PdfExportOptions.Parse(["원고.output.HWP"]);
        Check(options.OutputPath == Path.ChangeExtension(options.SourcePath, ".pdf"), "Default PDF path.");
        foreach (var args in new[] {
            new[] {"--output", "결과.pdf", "원고.hwp"},
            new[] {"--input", "원고.hwp", "결과.pdf"},
            new[] {"원고.hwp", "--output", "결과.pdf"},
        }) Check(PdfExportOptions.Parse(args).OutputPath == Path.GetFullPath("결과.pdf"), "PDF option order.");
        foreach (var args in new[] {
            Array.Empty<string>(), new[] {"input.hwpx"}, new[] {"input.hwp", "out.hwp"},
            new[] {"input.hwp", "a.pdf", "--output", "b.pdf"},
            new[] {"input.hwp", "--input", "other.hwp"},
            new[] {"--input", "input.hwp", "--input", "other.hwp"},
            new[] {"input.hwp", "--output"}, new[] {"input.hwp", "--output", "--help"},
        }) Reject(() => PdfExportOptions.Parse(args));
        var parameters = PdfExportFile.Parameters("out.pdf");
        Check((int)parameters["GraphicQuality"] == 100 && (int)parameters["Device"] == 5 &&
            (int)parameters["Range"] == 6 && (int)parameters["NumCopy"] == 1 &&
            (int)parameters["ZoomX"] == 100 && (int)parameters["ZoomY"] == 100, "Recommended native PDF parameters.");
        var directory = Path.Combine(Path.GetTempPath(), "md2hwp-pdf-test-" + Guid.NewGuid().ToString("N"));
        Directory.CreateDirectory(directory);
        var input = Path.Combine(directory, "source.hwp");
        var temporary = Path.Combine(directory, "temporary.pdf");
        var output = Path.Combine(directory, "existing.pdf");
        try
        {
            File.WriteAllText(input, "protected HWP");
            File.WriteAllText(output, "previous result");
            new PdfExportOptions(input, output).ValidatePaths();
            Reject(() => new PdfExportOptions(input, input).ValidatePaths());
            Reject(() => new PdfExportOptions(Path.Combine(directory, "missing.hwp"), output).ValidatePaths());
            Reject(() => new PdfExportOptions(input, Path.Combine(directory, "missing", "out.pdf")).ValidatePaths());
            Reject(() => new PdfExportOptions(input, directory).ValidatePaths());
            File.WriteAllText(temporary, "%PDF-1.4\nincomplete");
            Reject(() => PdfExportFile.Publish(temporary, output));
            Check(File.ReadAllText(output) == "previous result", "Failed export replaced the old PDF.");
            // A minimal envelope isolates completion checks; this is not a full semantic PDF fixture.
            var prefix = "%PDF-1.4\n1 0 obj\n<<>>\nendobj\n";
            var complete = prefix + "xref\n0 1\n0000000000 65535 f \ntrailer\n<< /Size 1 >>\nstartxref\n" +
                Encoding.ASCII.GetByteCount(prefix) + "\n%%EOF\n";
            foreach (var invalid in new[] {
                complete.Replace("%PDF-", "wrong"), complete.Replace("%%EOF", "unfinished"),
                complete.Replace("startxref", "nooffset"), complete.Replace("xref\n0 1", "bad!\n0 1"),
                complete + "unexpected trailing content",
                prefix + "xref\nstartxref\n999999999999999999999\n%%EOF\n",
            }) { File.WriteAllText(temporary, invalid); Reject(() => PdfExportFile.ValidateComplete(temporary)); }
            File.WriteAllText(temporary, complete);
            using (var lockedOutput = new FileStream(output, FileMode.Open, FileAccess.Read, FileShare.None))
                Reject(() => PdfExportFile.Publish(temporary, output));
            Check(File.ReadAllText(output) == "previous result" && File.Exists(temporary), "Locked output was replaced or temporary PDF lost.");
            var bytes = PdfExportFile.Publish(temporary, output);
            Check(bytes == Encoding.ASCII.GetByteCount(complete) && File.ReadAllText(output) == complete &&
                !File.Exists(temporary), "Completed export not published.");
            Check(File.ReadAllText(input) == "protected HWP", "PDF export touched the source.");
            Console.WriteLine("PDF CLI, native quality options, completion and failure-preserving replacement checks passed.");
        }
        finally
        {
            foreach (var path in new[] {input, temporary, output}) if (File.Exists(path)) File.Delete(path);
            Directory.Delete(directory);
        }
    }
}
