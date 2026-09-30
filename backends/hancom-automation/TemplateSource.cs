using System.Xml.Linq;

namespace Md2Hwp.HancomIrPreview;

internal sealed record TemplateSource(XElement Paragraph, string Slot, string Prefix, string Suffix)
{
    public string PrototypeText => TaggedTemplateBinding.DirectText(Paragraph);
    public string Render(string content) => Prefix + content + Suffix;
    public static TemplateSource Read(XElement paragraph, string slot)
    {
        if (!paragraph.Elements().Any() || paragraph.Elements().Any(e => e.Name.LocalName != "TEXT") ||
            paragraph.Elements().SelectMany(e => e.Elements()).Any(e => e.Name.LocalName != "CHAR" || e.HasElements))
            throw new InvalidDataException("Source must be one plain-text paragraph without controls or line breaks.");
        var text = TaggedTemplateBinding.DirectText(paragraph);
        var parts = text.Split(slot, StringSplitOptions.None);
        if (parts.Length != 2 || (parts[0] + parts[1]).Contains(TaggedTemplateBinding.Prefix, StringComparison.Ordinal))
            throw new InvalidDataException("Source requires exactly one content slot and no other declarations.");
        return new(new XElement(paragraph), slot, parts[0], parts[1]);
    }

    public void Verify(XElement paragraph, string content)
    {
        if (TaggedTemplateBinding.DirectText(paragraph) != Render(content) ||
            paragraph.Elements().Any(e => e.Name.LocalName != "TEXT") ||
            paragraph.Elements().SelectMany(e => e.Elements()).Any(e => e.Name.LocalName != "CHAR" || e.HasElements))
            throw new InvalidOperationException("Source paragraph differs from its template and IR content.");
        foreach (var attribute in Paragraph.Attributes().Where(a => a.Name.LocalName is not "InstId"))
            if ((string?)paragraph.Attribute(attribute.Name) != attribute.Value)
                throw new InvalidOperationException($"Source paragraph lost template attribute {attribute.Name}: expected {attribute.Value}, got {paragraph.Attribute(attribute.Name)?.Value}.");
    }
}

internal sealed class FigureSourcePrototype
{
    private readonly string nativeBlock;
    private readonly TemplateSource source;
    private FigureSourcePrototype(string nativeBlock, TemplateSource source) { this.nativeBlock = nativeBlock; this.source = source; }

    public static FigureSourcePrototype Bind(dynamic hwp, int root, TemplateSource source)
    {
        Move(hwp, root);
        Run(hwp, "MoveSelNextParaBegin");
        var block = (string)hwp.GetTextFile("HWP", "saveblock");
        Run(hwp, "Cancel");
        if (string.IsNullOrEmpty(block)) throw new InvalidOperationException("Empty source prototype block.");
        return new(block, source);
    }

    public void Insert(dynamic hwp, AuriPreviewStyleBindings styles, IReadOnlyList<PreviewTextRun> runs)
    {
        XDocument before = HwpMarkup.Parse((string)hwp.GetTextFile("HWPML2X", ""));
        var roots = AuriMinimalBoxPrototype.RootParagraphs(before);
        Run(hwp, "MoveDocEnd");
        object result = hwp.SetTextFile(nativeBlock, "HWP", "insertfile");
        if (result is not int status || status != 1) throw new InvalidOperationException("Source clone insertion failed.");
        XDocument after = HwpMarkup.Parse((string)hwp.GetTextFile("HWPML2X", ""));
        var cloned = AuriMinimalBoxPrototype.RootParagraphs(after);
        var index = roots.Count - 1;
        if (cloned.Count != roots.Count + 1 || TaggedTemplateBinding.DirectText(cloned[index]) != source.PrototypeText)
            throw new InvalidOperationException("Source clone must add exactly one paragraph at document end.");
        Move(hwp, index);
        Find(hwp, source.Slot);
        var sentinel = "MD2HWP_SOURCE_" + Guid.NewGuid().ToString("N");
        HancomPreviewWriter.InsertText(hwp, sentinel);
        XDocument marked = HwpMarkup.Parse((string)hwp.GetTextFile("HWPML2X", ""));
        if (TaggedTemplateBinding.DirectText(AuriMinimalBoxPrototype.RootParagraphs(marked)[index]) != source.PrototypeText.Replace(source.Slot, sentinel, StringComparison.Ordinal))
            throw new InvalidOperationException("Source slot selection escaped its clone.");
        Move(hwp, index); Find(hwp, sentinel); Run(hwp, "Delete");
        HancomPreviewWriter.InsertFormattedLine(hwp, runs, styles.Resolve("figure.source"));
        HancomPreviewWriter.RemoveHyperlinksInRoots(hwp, index, index + 1);
        XDocument saved = HwpMarkup.Parse((string)hwp.GetTextFile("HWPML2X", ""));
        source.Verify(AuriMinimalBoxPrototype.RootParagraphs(saved)[index], string.Concat(runs.Select(r => r.Text)));
        Run(hwp, "MoveDocEnd");
    }

    private static void Move(dynamic hwp, int root) { if (!(bool)hwp.SetPos(0, root, 0)) throw new InvalidOperationException("Cannot move to source paragraph."); Run(hwp, "MoveParaBegin"); }
    private static void Run(dynamic hwp, string action) { if (!(bool)hwp.HAction.Run(action)) throw new InvalidOperationException($"Source action failed: {action}"); }
    private static void Find(dynamic hwp, string text)
    {
        _ = hwp.HAction.GetDefault("RepeatFind", hwp.HParameterSet.HFindReplace.HSet);
        hwp.HParameterSet.HFindReplace.FindString = text;
        hwp.HParameterSet.HFindReplace.Direction = hwp.FindDir("Forward");
        hwp.HParameterSet.HFindReplace.IgnoreMessage = 1;
        if (!(bool)hwp.HAction.Execute("RepeatFind", hwp.HParameterSet.HFindReplace.HSet)) throw new InvalidOperationException("Source slot not found.");
    }
}
