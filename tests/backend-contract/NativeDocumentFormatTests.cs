using Md2Hwp.HancomIrPreview;
using System.Xml.Linq;

internal static class NativeDocumentFormatTests
{
    internal static void Run()
    {
        static void Check(bool condition, string message) { if (!condition) throw new Exception(message); }
        static void Reject(Action action)
        {
            try { action(); }
            catch (Exception error) when (error is ArgumentException or IOException) { return; }
            throw new Exception("Expected native document path rejection.");
        }
        Check(NativeDocumentFormat.FromPath("한글.HwPx") == "HWPX", "HWPX format must be case insensitive.");
        Check(NativeDocumentFormat.FromPath("한글.HWP") == "HWP", "HWP format must remain supported.");
        foreach (var path in new[] { "file", "file.xml", "file.hwpx.hwp.zip", "file.hwp." })
            Reject(() => NativeDocumentFormat.FromPath(path));
        XDocument Matrix(string value) => XDocument.Parse(
            $"<ROOT Keep='-1114.73747'><RENDERINGINFO><SCAMATRIX E1='1' E2='0' E3='{value}' E4='0' E5='1' E6='0' Other='-1114.73747'/></RENDERINGINFO><SCAMATRIX E3='-1114.73747'/></ROOT>");
        var originalMatrix = Matrix("-1114.73747");
        var originalCopy = new XDocument(originalMatrix);
        var normalizedMatrix = HancomPreviewWriter.NormalizeHwpxMatrixPrecision(originalMatrix);
        Check(XNode.DeepEquals(normalizedMatrix,
            HancomPreviewWriter.NormalizeHwpxMatrixPrecision(Matrix("-1114.73743"))), "Float32 HWPX serialization was rejected.");
        Check(!XNode.DeepEquals(normalizedMatrix,
            HancomPreviewWriter.NormalizeHwpxMatrixPrecision(Matrix("-1114.73847"))), "Different transform was accepted.");
        Check(XNode.DeepEquals(normalizedMatrix, HancomPreviewWriter.NormalizeHwpxMatrixPrecision(normalizedMatrix)),
            "HWPX matrix precision normalization must be idempotent.");
        Check(XNode.DeepEquals(originalMatrix, originalCopy) &&
            (string?)normalizedMatrix.Root!.Attribute("Keep") == "-1114.73747" &&
            (string?)normalizedMatrix.Descendants("SCAMATRIX").First().Attribute("Other") == "-1114.73747" &&
            (string?)normalizedMatrix.Root.Element("SCAMATRIX")!.Attribute("E3") == "-1114.73747",
            "HWPX precision normalization modified input or unrelated attributes.");
        foreach (var nonfinite in new[] { "NaN", "Infinity", "1e100", "text" })
        {
            try { _ = HancomPreviewWriter.NormalizeHwpxMatrixPrecision(Matrix(nonfinite)); }
            catch (InvalidOperationException) { continue; }
            throw new Exception("Nonfinite or malformed HWPX transform accepted.");
        }
        XDocument Images(string firstId, string secondId, string secondData = "Ag==") => XDocument.Parse(
            $"<ROOT><P><IMAGE BinItem='{firstId}'/><IMAGE BinItem='{secondId}'/></P><BINDATA Id='{firstId}'>AQ==</BINDATA><BINDATA Id='{secondId}'>{secondData}</BINDATA></ROOT>");
        var oldImages = Images("4", "1");
        var oldImagesCopy = new XDocument(oldImages);
        var normalizedImages = HancomPreviewWriter.NormalizeHwpxImageIdentities(oldImages);
        var renumberedImages = HancomPreviewWriter.NormalizeHwpxImageIdentities(Images("1", "2"));
        Check(XNode.DeepEquals(normalizedImages, renumberedImages), "HWPX image identity renumbering changed content.");
        Check(!XNode.DeepEquals(normalizedImages.Element("ROOT")!.Element("P"),
            HancomPreviewWriter.NormalizeHwpxImageIdentities(Images("1", "2", "Aw==")).Root!.Element("P")),
            "Changed image bytes were hidden by identity normalization.");
        var duplicateImages = XDocument.Parse("<ROOT><P><IMAGE BinItem='1'/><IMAGE BinItem='2'/></P><BINDATA Id='1'>AQ==</BINDATA><BINDATA Id='2'>AQ==</BINDATA></ROOT>");
        var deduplicatedImages = XDocument.Parse("<ROOT><P><IMAGE BinItem='3'/><IMAGE BinItem='3'/></P><BINDATA Id='3'>AQ==</BINDATA></ROOT>");
        Check(XNode.DeepEquals(HancomPreviewWriter.NormalizeHwpxImageIdentities(duplicateImages),
            HancomPreviewWriter.NormalizeHwpxImageIdentities(deduplicatedImages)), "Identical image deduplication lost references.");
        Check(XNode.DeepEquals(oldImages, oldImagesCopy) &&
            XNode.DeepEquals(normalizedImages, HancomPreviewWriter.NormalizeHwpxImageIdentities(normalizedImages)),
            "Image identity normalization mutated input or was not idempotent.");
        var directory = Path.Combine(Path.GetTempPath(), "md2hwp-format-" + Guid.NewGuid().ToString("N"));
        Directory.CreateDirectory(directory);
        try
        {
            var source = Path.Combine(directory, "template.hwpx");
            File.WriteAllText(source, "protected source");
            var expectedSource = File.ReadAllBytes(source);
            using (var writer = new FileStream(source, FileMode.Open, FileAccess.ReadWrite, FileShare.ReadWrite))
            {
                var fingerprint = InvestigationTemplateProfile.ReadTemplateFingerprint(source);
                Check(fingerprint.Bytes == expectedSource.Length && fingerprint.Sha256 ==
                    Convert.ToHexString(System.Security.Cryptography.SHA256.HashData(expectedSource)),
                    "Template fingerprint must allow Hancom's existing HWPX write handle.");
                Check(writer.Length == expectedSource.Length, "Fingerprint changed the open template.");
            }
            Check(File.ReadAllBytes(source).SequenceEqual(expectedSource), "Fingerprint modified template bytes.");
            foreach (var extension in new[] { ".hwp", ".HWPX" })
            {
                var output = Path.Combine(directory, "result" + extension);
                Check(HancomPreviewWriter.ValidateRenderedOutput(output, source) == output, "Native output rejected.");
                var temporary = NativeDocumentFormat.TemporaryPath(output, "contract");
                Check(Path.GetDirectoryName(temporary) == directory &&
                    NativeDocumentFormat.FromPath(temporary) == NativeDocumentFormat.FromPath(output) &&
                    temporary != NativeDocumentFormat.TemporaryPath(output, "contract"), "Temporary format, location or uniqueness lost.");
                File.WriteAllText(output, "old result");
                Reject(() => HancomPreviewWriter.PublishRenderedOutput(Path.Combine(directory, "missing" + extension), output));
                Check(File.ReadAllText(output) == "old result", "Failed publication replaced the old result.");
                Reject(() => HancomPreviewWriter.ValidateRenderedOutput(output, output));
            }
            Reject(() => HancomPreviewWriter.ValidateRenderedOutput(source, source));
            Reject(() => HancomPreviewWriter.ValidateRenderedOutput(Path.Combine(directory, "result.pdf"), source));
            Check(File.ReadAllText(source) == "protected source", "Source was changed by path validation.");
        }
        finally { Directory.Delete(directory, recursive: true); }
    }
}
