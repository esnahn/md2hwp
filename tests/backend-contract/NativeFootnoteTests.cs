using System.Text.Json;
using System.Xml.Linq;
using Md2Hwp.HancomIrPreview;

internal static class NativeFootnoteTests
{
    internal static void Run()
    {
        var source = Fixture("OnPage");
        var original = source.ToString();
        var layout = NativeFootnotes.Bind(source);
        using var input = JsonDocument.Parse("""[{"type":"footnote","blocks":[{"type":"paragraph","inlines":[{"type":"text","value":"생성각주"}]}]}]""");
        var content = InlineText.Read(input.RootElement, "/inlines");
        var plan = new IrPreviewPlan("fixture", "fixture", new(1, 1, 0, 0, 0),
            [new("text", "body", [content.Lines.Single().Text], ParagraphStyle: "body",
                FormattedLines: content.Lines.Select(line => line.Runs).ToArray())], []);
        var rendered = new XDocument(source);
        var section = rendered.Descendants("SECTION").Single();
        section.Elements("P").Where(paragraph => TaggedTemplateBinding.DirectText(paragraph).StartsWith("{{md2hwp:", StringComparison.Ordinal)).Remove();
        var generatedHost = Paragraph("앞 " + content.Lines.Single().Text + " 뒤");
        section.Add(generatedHost);
        var attached = layout.Attach(rendered, plan);
        Check(source.ToString() == original, "Footnote identity tracking mutated its source prototype.");
        var note = attached.Descendants("FOOTNOTE").Last();
        var first = note.Element("PARALIST")!.Elements("P").First();
        var identity = (string?)first.Attribute("InstId");
        Check(!string.IsNullOrWhiteSpace(identity), "Generated footnotes require a stable native paragraph identity.");

        // Reference formatting can insert TEXT runs before a note's existing
        // run. Its cached path must be recaptured from the final attached XML.
        var hostText = note.Parent!;
        var hostParagraph = hostText.Parent!;
        note.Remove();
        hostText.ReplaceWith(
            new XElement("TEXT", new XAttribute("CharShape", 0), new XElement("CHAR", "앞 ")),
            new XElement("TEXT", new XAttribute("CharShape", 1), new XElement("CHAR", "그림 3-1")),
            new XElement("TEXT", new XAttribute("CharShape", 0), note, new XElement("CHAR", " 뒤")));
        var expected = new XDocument(attached);
        layout.RecordLayout(expected);
        var actual = new XDocument(expected);
        actual.Descendants("FOOTNOTE").First().Descendants("AUTONUM").Single().SetAttributeValue("Number", 999);
        actual.Descendants("FOOTNOTE").Last().Descendants("AUTONUM").Single().SetAttributeValue("Number", 1);
        layout.NormalizeNumbers(expected, actual);
        Check((int?)actual.Descendants("FOOTNOTE").Last().Descendants("AUTONUM").Single().Attribute("Number") == 2,
            "A note moved by reference-run splitting escaped OnPage numbering normalization.");
        Check((int?)actual.Descendants("FOOTNOTE").First().Descendants("AUTONUM").Single().Attribute("Number") == 999,
            "OnPage normalization altered an unrelated existing note.");

        var furtherSplit = new XDocument(expected);
        furtherSplit.Descendants("FOOTNOTE").Last().Parent!.AddBeforeSelf(
            new XElement("TEXT", new XAttribute("CharShape", 1), new XElement("CHAR", "another reference")));
        layout.RecordLayout(furtherSplit);
        var saved = new XDocument(furtherSplit);
        saved.Descendants("FOOTNOTE").Last().Descendants("AUTONUM").Single().SetAttributeValue("Number", 1);
        layout.NormalizeNumbers(furtherSplit, saved);
        Check((int?)saved.Descendants("FOOTNOTE").Last().Descendants("AUTONUM").Single().Attribute("Number") == 2,
            "Repeated final layout capture retained a stale footnote path.");

        var lost = new XDocument(expected);
        lost.Descendants("FOOTNOTE").Last().Element("PARALIST")!.Element("P")!.Attribute("InstId")!.Value = "lost";
        Reject(() => layout.RecordLayout(lost));
        var duplicate = new XDocument(expected);
        duplicate.Descendants("SECTION").Single().Add(Paragraph("", new XElement(duplicate.Descendants("FOOTNOTE").Last())));
        Reject(() => layout.RecordLayout(duplicate));
        var missingNumber = new XDocument(expected);
        missingNumber.Descendants("FOOTNOTE").Last().Descendants("AUTONUM").Remove();
        Reject(() => layout.RecordLayout(missingNumber));

        var continuousSource = Fixture("Continuous");
        var continuous = NativeFootnotes.Bind(continuousSource);
        var continuousRendered = new XDocument(rendered);
        continuousRendered.Descendants("NOTENUMBERING").Single().SetAttributeValue("Type", "Continuous");
        var continuousExpected = continuous.Attach(continuousRendered, plan);
        continuous.RecordLayout(continuousExpected);
        var continuousActual = new XDocument(continuousExpected);
        continuousActual.Descendants("FOOTNOTE").Last().Descendants("AUTONUM").Single().SetAttributeValue("Number", 100);
        continuous.NormalizeNumbers(continuousExpected, continuousActual);
        Check((int?)continuousActual.Descendants("FOOTNOTE").Last().Descendants("AUTONUM").Single().Attribute("Number") == 100,
            "Continuous numbering was normalized as page restarts.");
        Console.WriteLine("Generated OnPage note counters follow final paragraph identities after reference-run changes.");
    }

    private static XElement Paragraph(string text, XElement? control = null) => new("P",
        new XAttribute("Style", 0), new XAttribute("ParaShape", 0),
        new XElement("TEXT", new XAttribute("CharShape", 0), control, new XElement("CHAR", text)));

    private static XDocument Fixture(string numbering) => new(new XElement("HWPML",
        new XElement("HEAD",
            new XElement("PARASHAPELIST", new XAttribute("Count", 1), new XElement("PARASHAPE", new XAttribute("Id", 0))),
            new XElement("CHARSHAPELIST", new XAttribute("Count", 2),
                new XElement("CHARSHAPE", new XAttribute("Id", 0), new XAttribute("Height", 1000)),
                new XElement("CHARSHAPE", new XAttribute("Id", 1), new XAttribute("Height", 900)))),
        new XElement("DOCSETTING", new XElement("BEGINNUMBER", new XAttribute("Footnote", 1))),
        new XElement("BODY", new XElement("SECTION",
            Paragraph("", new XElement("SECDEF", new XElement("FOOTNOTESHAPE",
                new XElement("AUTONUMFORMAT", new XAttribute("Type", "Digit"), new XAttribute("SuffixChar", ")")),
                new XElement("NOTENUMBERING", new XAttribute("Type", numbering), new XAttribute("NewNumber", 1))))),
            Paragraph("static", new XElement("FOOTNOTE", new XElement("PARALIST",
                new XElement("P", new XAttribute("InstId", "static-note"), new XAttribute("Style", 0), new XAttribute("ParaShape", 0),
                    new XElement("TEXT", new XAttribute("CharShape", 0),
                        new XElement("AUTONUM", new XAttribute("NumberType", "Footnote"), new XAttribute("Number", 1),
                            new XElement("AUTONUMFORMAT", new XAttribute("Type", "Digit"))),
                        new XElement("CHAR", "existing")))))),
            Paragraph(TaggedTemplateBinding.Tag("begin:template")), Paragraph(TaggedTemplateBinding.Tag("footnote")),
            Paragraph(TaggedTemplateBinding.Tag("end:template"))))));

    private static void Check(bool value, string message) { if (!value) throw new Exception(message); }
    private static void Reject(Action action)
    {
        try { action(); } catch (InvalidOperationException) { return; }
        throw new Exception("Expected ambiguous or lost generated footnote rejection.");
    }
}
