using System.Xml.Linq;
using Md2Hwp.HancomIrPreview;

internal static class TableWidthTests
{
    internal static void Run()
    {
        Check(TableAutoWidths.Distribute([1000, 1000], [1000, 3000], 3000).SequenceEqual(new[] { 1000, 2000 }),
            "Measured table fitting gave space to an already sufficient short column.");
        var spread = TableAutoWidths.Distribute([1000, 1000, 1000], [1800, 3000, 10000], 14000);
        Check(spread.Sum() == 14000 && spread[0] < spread[1] && spread[1] < spread[2],
            "Content-based widths were not ordered by measured preferred sizes.");
        var fractional = TableAutoWidths.Distribute([10.2, 10.2, 10.2], [100.1, 100.1, 100.1], 100);
        Check(fractional.SequenceEqual(new[] { 34, 33, 33 }), "Table width rounding is not exact and deterministic.");
        Reject(() => TableAutoWidths.Distribute([1000, 1000], [3000, 3000], 1999), "minimum");
        Reject(() => TableAutoWidths.Distribute([double.NaN], [1], 100), "Invalid");

        var document = FontFixture();
        var shape = document.Descendants("CHARSHAPE").Single();
        var plain = TableAutoWidths.MeasurePlainText(document, shape, "iiiiMMMM0123");
        var doubled = new XElement(shape); doubled.SetAttributeValue("Height", 2000);
        var large = TableAutoWidths.MeasurePlainText(document, doubled, "iiiiMMMM0123");
        Check(plain > 0 && Math.Abs(large / plain - 2) < 0.01, "Native font measurement did not preserve font size scaling.");
        var narrow = new XElement(shape);
        foreach (var ratio in narrow.Element("RATIO")!.Attributes()) ratio.Value = "50";
        Check(Math.Abs(TableAutoWidths.MeasurePlainText(document, narrow, "iiiiMMMM0123") / plain - 0.5) < 0.001,
            "Native font measurement ignored horizontal scale.");
        var spaced = new XElement(shape);
        foreach (var spacing in spaced.Element("CHARSPACING")!.Attributes()) spacing.Value = "20";
        Check(TableAutoWidths.MeasurePlainText(document, spaced, "iiiiMMMM0123") > plain,
            "Native font measurement ignored character spacing.");
        var oneCharacter = TableAutoWidths.MeasurePlainText(document, shape, "i");
        var oneSpaced = TableAutoWidths.MeasurePlainText(document, spaced, "i");
        var fourSpaced = TableAutoWidths.MeasurePlainText(document, spaced, "iiii");
        Check(Math.Abs(oneSpaced - oneCharacter) < 0.001 &&
            Math.Abs(fourSpaced - oneCharacter * (4 + 3 * 0.2)) < 0.001,
            "Character spacing was added after the last glyph or used the font em instead of glyph advance.");
        Check(Math.Abs(TableAutoWidths.MeasurePlainText(document, shape, " ") - 500) < 0.001,
            "The native half-em space was replaced with the font's natural space width.");
        var naturalSpace = new XElement(shape); naturalSpace.SetAttributeValue("UseFontSpace", "true");
        Check(TableAutoWidths.MeasurePlainText(document, naturalSpace, " ") < 500,
            "Font-space selection did not use the actual space glyph.");
        Check(TableAutoWidths.MeasurePlainText(document, shape, "iiii", strong: true) >
            TableAutoWidths.MeasurePlainText(document, shape, "iiii"), "Native font measurement ignored strong formatting.");
        var unsupportedFont = FontFixture();
        foreach (var font in unsupportedFont.Descendants("FONT")) font.SetAttributeValue("Type", "hft");
        Reject(() => TableAutoWidths.MeasurePlainText(unsupportedFont, shape, "text"), "HFT");
        var missingFont = FontFixture();
        foreach (var font in missingFont.Descendants("FONT")) font.SetAttributeValue("Name", "md2hwpMissingFontForWidthTest");
        Reject(() => TableAutoWidths.MeasurePlainText(missingFont, shape, "text"), "unavailable");
        var widths = TableAutoWidths.Calculate(Table(), document, Prototype(), 30000);
        Check(widths.Sum() == 30000 && widths[0] < widths[1] && widths[1] < widths[2],
            "Measured ID/date/description columns were not automatically sized by their content.");
        var withSource = Table() with { Source = PreviewInlineContent.Plain([new string('M', 1000)]) };
        Check(TableAutoWidths.Calculate(withSource, document, Prototype(), 30000).SequenceEqual(widths),
            "A merged table source influenced individual column widths.");
        var padding = Prototype();
        foreach (var margin in padding.Descendants("CELLMARGIN")) { margin.SetAttributeValue("Left", 1200); margin.SetAttributeValue("Right", 1200); }
        Reject(() => TableAutoWidths.Calculate(Table(), document, padding, 5000), "minimum");
        var native = new PreviewTable(["default"], [PreviewInlineContent.Plain([""])],
            new[] { new[] { new PreviewInlineContent([new PreviewLine("GUID_MARKER", [new PreviewTextRun("GUID_MARKER", false, false,
                CrossReference: new PreviewCrossReference("figure_number", "target"))])]) } });
        var otherMarker = native with { Rows = new[] { new[] { new PreviewInlineContent([new PreviewLine(new string('M', 2000),
            [new PreviewTextRun(new string('M', 2000), false, false, CrossReference: new PreviewCrossReference("figure_number", "target"))])]) } } };
        Check(TableAutoWidths.Calculate(native, document, Prototype(), 15000)
            .SequenceEqual(TableAutoWidths.Calculate(otherMarker, document, Prototype(), 15000)),
            "Internal reference marker length leaked into the table's measured widths.");
        var empty = new PreviewTable(["default"], [PreviewInlineContent.Plain([""])],
            new[] { new[] { PreviewInlineContent.Plain([""]) } });
        var indented = FontFixture();
        indented.Descendants("PARAMARGIN").Single().SetAttributeValue("Left", 2000);
        Check(TableAutoWidths.Calculate(empty, indented, Prototype(), 1200).Single() == 1200,
            "Legacy HWPML paragraph URC margins were not converted to HWPUNIT.");
        Console.WriteLine("Content-based table widths, native font metrics, margins and exact rounding checks passed.");
    }

    private static PreviewTable Table() => new(["default", "default", "default"],
        new[] { PreviewInlineContent.Plain(["ID"]), PreviewInlineContent.Plain(["Date"]), PreviewInlineContent.Plain(["Description"]) },
        new[] { new[] { PreviewInlineContent.Plain(["1"]), PreviewInlineContent.Plain(["2026-10-05"]),
            PreviewInlineContent.Plain(["The table contains a longer description with words that wrap."]) } });

    private static XDocument FontFixture()
    {
        string[] languages = ["Hangul", "Latin", "Hanja", "Japanese", "Other", "Symbol", "User"];
        XElement Values(string name, int value) => new(name, languages.Select(language => new XAttribute(language, value)));
        var faces = new XElement("FACENAMELIST", languages.Select(language =>
            new XElement("FONTFACE", new XAttribute("Lang", language),
                new XElement("FONT", new XAttribute("Id", "0"), new XAttribute("Name", "Arial")))));
        var shapes = new XElement("CHARSHAPELIST", new XElement("CHARSHAPE", new XAttribute("Id", "0"),
            new XAttribute("Height", 1000), Values("FONTID", 0), Values("RATIO", 100), Values("RELSIZE", 100), Values("CHARSPACING", 0)));
        var paragraphs = new XElement("PARASHAPELIST", new XElement("PARASHAPE", new XAttribute("Id", "0"),
            new XAttribute("BreakLatinWord", "KeepWord"), new XAttribute("BreakNonLatinWord", "true"),
            new XElement("PARAMARGIN", new XAttribute("Indent", 0), new XAttribute("Left", 0), new XAttribute("Right", 0))));
        return new(new XElement("HWPML", new XElement("HEAD", new XElement("MAPPINGTABLE", faces, shapes, paragraphs))));
    }

    private static XElement Prototype() => new("TABLE",
        new XElement("INSIDEMARGIN", new XAttribute("Left", 0), new XAttribute("Right", 0)),
        Enumerable.Range(0, 3).Select(_ => new XElement("ROW", new XElement("CELL", new XAttribute("HasMargin", "true"),
            new XElement("CELLMARGIN", new XAttribute("Left", 100), new XAttribute("Right", 100)),
            new XElement("PARALIST", new XElement("P", new XAttribute("ParaShape", "0"),
                new XElement("TEXT", new XAttribute("CharShape", "0"), new XElement("CHAR", "slot"))))))));

    private static void Check(bool condition, string message)
    {
        if (!condition) throw new InvalidOperationException(message);
    }
    private static void Reject(Action action, string message)
    {
        try { action(); }
        catch (InvalidDataException exception) when (exception.Message.Contains(message, StringComparison.OrdinalIgnoreCase)) { return; }
        throw new InvalidOperationException("Expected table width error: " + message);
    }
}
