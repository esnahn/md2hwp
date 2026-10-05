using System.Xml.Linq;

namespace Md2Hwp.HancomIrPreview;

internal readonly record struct ParagraphProbePosition(int List, int Paragraph, int Offset);

// A narrow COM seam also lets contract tests exercise cleanup failures without Hancom.
internal interface IParagraphStyleProbe
{
    void Run(string action);
    ParagraphProbePosition Position();
    bool TrackingChanges { get; }
    void InsertMarker();
    bool Select(ParagraphProbePosition begin, ParagraphProbePosition end);
    int SelectionMode { get; }
    XDocument ReadBlock();
    XDocument ReadDocument();
}

internal static class CurrentParagraphStyle
{
    internal const string Marker = "x";

    internal static string Read(object automation) => Read(new HancomProbe(automation));

    internal static string Read(IParagraphStyleProbe probe)
    {
        probe.Run("Cancel");
        probe.Run("MoveDocEnd");
        var begin = probe.Position();
        if (begin.List != 0 || begin.Paragraph < 0 || begin.Offset < 0 || (probe.SelectionMode & 0x0f) != 0)
            throw new InvalidOperationException("Style inspection did not reach the final root paragraph.");
        // A marker would become a tracked edit. Preserve the original full-read
        // path for documents recording changes, without changing that setting.
        if (probe.TrackingChanges) return LastStyleName(probe.ReadDocument());
        var end = begin with { Offset = checked(begin.Offset + Marker.Length) };
        bool inserted = false;
        string? name = null;
        Exception? failure = null;
        try
        {
            // HWP 2020 saveblock synthesizes a default-style terminal empty P,
            // even when two paragraphs are selected. One temporary character
            // makes the actual paragraph's named style exportable. Select only
            // that character, never preceding tables or their nested controls.
            probe.InsertMarker();
            inserted = true;
            if (probe.Position() != end)
                throw new InvalidOperationException("Style probe insertion changed unexpected native coordinates.");
            if (!probe.Select(begin, end) || (probe.SelectionMode & 0x0f) != 1)
                throw new InvalidOperationException("Could not select the paragraph style probe character.");
            name = SelectedStyleName(probe.ReadBlock());
        }
        catch (Exception error) { failure = error; }

        try
        {
            probe.Run("Cancel");
            probe.Run("MoveDocEnd");
            if (inserted)
            {
                // Never backspace an original character or a paragraph boundary.
                if (probe.Position() != end || (probe.SelectionMode & 0x0f) != 0)
                    throw new InvalidOperationException("Could not safely locate the paragraph style probe for removal.");
                probe.Run("DeleteBack");
            }
            if (probe.Position() != begin || (probe.SelectionMode & 0x0f) != 0)
                throw new InvalidOperationException("Paragraph style inspection did not restore its original cursor.");
        }
        catch (Exception error)
        {
            failure = failure is null ? error : new AggregateException(failure, error);
        }
        if (failure is not null)
            throw new InvalidOperationException("Could not inspect and restore the current paragraph style.", failure);
        return name!;
    }

    internal static string SelectedStyleName(XDocument document)
    {
        var roots = AuriMinimalBoxPrototype.RootParagraphs(document);
        if (roots.Count != 1 || TaggedTemplateBinding.DirectText(roots[0]) != Marker ||
            roots[0].Elements().Any(e => e.Name.LocalName != "TEXT") ||
            roots[0].Elements().Elements().Any(e => e.Name.LocalName is not ("CHAR" or "SECDEF" or "COLDEF")) ||
            roots[0].Elements().Elements().Where(e => e.Name.LocalName == "CHAR").Any(e => e.Nodes().Any(n => n is not XText)))
            throw new InvalidDataException("Paragraph style export did not contain exactly the selected probe character.");
        return StyleName(document, roots[0]);
    }

    private static string LastStyleName(XDocument document)
    {
        var roots = AuriMinimalBoxPrototype.RootParagraphs(document);
        if (roots.Count == 0) throw new InvalidDataException("Style inspection found no root paragraph.");
        return StyleName(document, roots[^1]);
    }

    private static string StyleName(XDocument document, XElement paragraph)
    {
        var id = (string?)paragraph.Attribute("Style");
        if (!int.TryParse(id, out var number) || number < 0)
            throw new InvalidDataException("Style inspection found an invalid paragraph style ID.");
        var styles = document.Descendants().Where(e => e.Name.LocalName == "STYLE" && (string?)e.Attribute("Id") == id).ToArray();
        if (styles.Length != 1 || (string?)styles[0].Attribute("Type") != "Para" ||
            string.IsNullOrEmpty((string?)styles[0].Attribute("Name")))
            throw new InvalidDataException("Style inspection found a missing or ambiguous named paragraph style.");
        // saveblock renumbers style IDs. The original document's target ID is
        // used only for application; identity is compared by exact native name.
        return (string)styles[0].Attribute("Name")!;
    }

    private sealed class HancomProbe(object automation) : IParagraphStyleProbe
    {
        private readonly dynamic hwp = automation;
        public bool TrackingChanges => Convert.ToBoolean((object)hwp.IsTrackChange, System.Globalization.CultureInfo.InvariantCulture);
        public int SelectionMode => (int)hwp.SelectionMode;
        public void Run(string action)
        {
            if (!(bool)hwp.HAction.Run(action))
                throw new InvalidOperationException($"Hancom paragraph style probe action failed: {action}");
        }
        public ParagraphProbePosition Position()
        {
            int list, paragraph, offset;
            hwp.GetPos(out list, out paragraph, out offset);
            return new(list, paragraph, offset);
        }
        public void InsertMarker() => HancomPreviewWriter.InsertText((object)hwp, Marker);
        public bool Select(ParagraphProbePosition begin, ParagraphProbePosition end) =>
            (bool)hwp.SelectText(begin.Paragraph, begin.Offset, end.Paragraph, end.Offset);
        public XDocument ReadBlock() => RenderProfile.ReadBlock((object)hwp);
        public XDocument ReadDocument() => RenderProfile.ReadDocument((object)hwp);
    }
}