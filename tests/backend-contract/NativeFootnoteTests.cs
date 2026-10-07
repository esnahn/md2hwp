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
        TestNativeSample();
        Console.WriteLine("Generated OnPage note counters follow final paragraph identities after reference-run changes.");
    }

    private static void TestNativeSample()
    {
        var fixture = Fixture("Continuous");
        var sample = fixture.Descendants("P").Single(p => TaggedTemplateBinding.DirectText(p) == TaggedTemplateBinding.Tag("footnote"));
        var number = new XElement("AUTONUM", new XAttribute("NumberType", "Footnote"), new XAttribute("Number", 23),
            new XElement("AUTONUMFORMAT", new XAttribute("Type", "Digit"), new XAttribute("SuffixChar", ")")));
        var native = new XElement("FOOTNOTE", new XAttribute("InstId", "sample-note"),
            new XElement("PARALIST", new XAttribute("LineWrap", "Break"), new XAttribute("VertAlign", "Top"),
                new XAttribute("TextDirection", "0"), sample));
        var anchor = Paragraph("각주 원형", native);
        sample.ReplaceWith(anchor);
        sample = native.Descendants("P").Single();
        sample.Element("TEXT")!.Element("CHAR")!.Value = "{{md2hwp:foot";
        sample.Element("TEXT")!.Element("CHAR")!.AddFirst(new XElement("TAB", new XAttribute("Width", 720)));
        sample.Add(new XElement("TEXT", new XAttribute("CharShape", 1), new XElement("CHAR", "note}}")));
        sample.AddFirst(new XElement("TEXT", new XAttribute("CharShape", 1), number, new XElement("CHAR", " ")));
        var original = fixture.ToString();
        var binding = NativeFootnotes.Bind(fixture);
        var lowered = binding.LowerSample(fixture);
        Check(lowered.Descendants("FOOTNOTE").Count() == 1 &&
            AuriMinimalBoxPrototype.RootParagraphs(lowered).Any(p => TaggedTemplateBinding.DirectText(p) == TaggedTemplateBinding.Tag("footnote")),
            "Native sample anchor was not flattened or an unrelated existing note was removed.");
        using var input = JsonDocument.Parse("""[{"type":"footnote","blocks":[{"type":"paragraph","inlines":[{"type":"strong","inlines":[{"type":"text","value":"각주"},{"type":"space"},{"type":"text","value":"본문"}]}]},{"type":"paragraph","inlines":[{"type":"text","value":"둘째"},{"type":"space"},{"type":"text","value":"문단"}]}]}]""");
        var content = InlineText.Read(input.RootElement, "/inlines");
        var plan = new IrPreviewPlan("fixture", "fixture", new(1, 1, 0, 0, 0),
            [new("text", "body", [content.Lines.Single().Text], ParagraphStyle: "body",
                FormattedLines: content.Lines.Select(line => line.Runs).ToArray())], []);
        var rendered = new XDocument(lowered);
        rendered.Descendants("SECTION").Single().Elements("P").Where(p =>
            TaggedTemplateBinding.DirectText(p).StartsWith("{{md2hwp:", StringComparison.Ordinal)).Remove();
        rendered.Descendants("SECTION").Single().Add(Paragraph(content.Lines.Single().Text));
        var result = binding.Attach(rendered, plan);
        var note = result.Descendants("FOOTNOTE").Last();
        Check(note.Element("PARALIST")!.Attributes().Select(a => (a.Name, a.Value)).SequenceEqual(native.Element("PARALIST")!.Attributes().Select(a => (a.Name, a.Value))),
            "Native footnote paragraph-list options were not preserved.");
        var paragraphs = note.Element("PARALIST")!.Elements("P").ToArray();
        var generatedNumber = paragraphs[0].Descendants("AUTONUM").Single();
        Check((int?)generatedNumber.Attribute("Number") == 2 && XNode.DeepEquals(generatedNumber.Element("AUTONUMFORMAT"), number.Element("AUTONUMFORMAT")) &&
            (string?)generatedNumber.Parent!.Attribute("CharShape") == "1",
            "Native sample number format, number character formatting or continuous counter was lost.");
        Check(paragraphs.Length == 2 && !paragraphs[1].Descendants("AUTONUM").Any() &&
            TaggedTemplateBinding.DirectText(paragraphs[0]) == " 각주 본문" && TaggedTemplateBinding.DirectText(paragraphs[1]) == "둘째 문단",
            "Native sample lost note paragraphs or repeated its number/separator.");
        Check(paragraphs[0].Descendants("TAB").Count() == 1 &&
            (int?)paragraphs[0].Descendants("TAB").Single().Attribute("Width") == 720 && !paragraphs[1].Descendants("TAB").Any(),
            "The native sample's tab separator was dropped, changed or copied to continuation paragraphs.");
        var bodyRun = paragraphs[0].Elements("TEXT").Single(run => run.Value.Contains("각주 본문", StringComparison.Ordinal));
        Check((int?)result.Descendants("CHARSHAPE").Single(shape =>
            (string?)shape.Attribute("Id") == (string?)bodyRun.Attribute("CharShape")).Attribute("Height") == 1000,
            "A split native body slot inherited number or trailing tag formatting instead of its first character.");
        Check(fixture.ToString() == original, "Native note binding/lowering changed the source.");
        foreach (var rootSample in new[] { false, true })
        {
            var separate = new XDocument(fixture);
            var firstSample = separate.Descendants("P").Single(p => TaggedTemplateBinding.DirectText(p).Trim() == TaggedTemplateBinding.Tag("footnote"));
            var continuation = Paragraph(TaggedTemplateBinding.Tag("footnote.next"));
            continuation.Element("TEXT")!.SetAttributeValue("CharShape", "1");
            var shape = new XElement(separate.Descendants("PARASHAPE").First());
            shape.SetAttributeValue("Id", "99"); shape.SetAttributeValue("ContinuationIndent", "720");
            separate.Descendants("PARASHAPE").First().Parent!.Add(shape);
            continuation.SetAttributeValue("ParaShape", "99");
            if (rootSample) firstSample.Ancestors("P").Last().AddAfterSelf(continuation);
            else firstSample.AddAfterSelf(continuation);
            var separateBefore = separate.ToString();
            var separateBinding = NativeFootnotes.Bind(separate);
            var separateRendered = separateBinding.LowerSample(separate);
            Check(!separateRendered.Descendants("P").Any(p => TaggedTemplateBinding.DirectText(p) == TaggedTemplateBinding.Tag("footnote.next")),
                "Continuation declaration survived lowering.");
            separateRendered.Descendants("SECTION").Single().Elements("P").Where(p =>
                TaggedTemplateBinding.DirectText(p).StartsWith("{{md2hwp:", StringComparison.Ordinal)).Remove();
            separateRendered.Descendants("SECTION").Single().Add(Paragraph(content.Lines.Single().Text));
            var separateRun = content.Lines.Single().Runs.Single();
            var separateNote = separateRun.Footnote!;
            var extendedRun = separateRun with { Footnote = new PreviewFootnote(separateNote.Paragraphs.Append(separateNote.Paragraphs.Last()).ToArray()) };
            var extendedOperation = plan.Operations.Single() with { FormattedLines = new[] { new[] { extendedRun } } };
            var separateResult = separateBinding.Attach(separateRendered, plan with { Operations = new[] { extendedOperation } });
            var continued = separateResult.Descendants("FOOTNOTE").Last().Element("PARALIST")!.Elements("P").ToArray();
            var continuationShape = separateResult.Descendants("PARASHAPE").Single(p => (string?)p.Attribute("Id") == (string?)continued[1].Attribute("ParaShape"));
            Check((string?)continuationShape.Attribute("ContinuationIndent") == "720" &&
                (string?)continued[1].Element("TEXT")!.Attribute("CharShape") == "1" &&
                !continued[1].Descendants("AUTONUM").Any() && !continued[1].Descendants("TAB").Any(),
                "Second note paragraph lost its separate full formatting or inherited number/tab controls.");
            Check(continued.Length == 3 && (string?)continued[2].Attribute("ParaShape") == (string?)continued[1].Attribute("ParaShape") &&
                (string?)continued[2].Element("TEXT")!.Attribute("CharShape") == "1" && !continued[2].Descendants("AUTONUM").Any(),
                "Third and later note paragraphs did not repeat the continuation format.");
            Check(separate.ToString() == separateBefore, "Continuation sample binding mutated its source.");
            var decorated = new XDocument(separate);
            var decoratedSample = decorated.Descendants("P").Single(p => TaggedTemplateBinding.DirectText(p) == TaggedTemplateBinding.Tag("footnote.next"));
            var decoratedRun = decoratedSample.Element("TEXT")!;
            decoratedRun.Element("CHAR")!.ReplaceNodes("앞", new XElement("TAB", new XAttribute("Width", 720)), TaggedTemplateBinding.Tag("footnote.next"), "뒤");
            decoratedRun.AddFirst(new XElement(number));
            var trailingNumber = new XElement(number); trailingNumber.SetAttributeValue("NumberType", "Figure"); trailingNumber.SetAttributeValue("Number", 7);
            decoratedRun.Add(trailingNumber);
            var decoratedBefore = decorated.ToString();
            var decoratedBinding = NativeFootnotes.Bind(decorated);
            var decoratedRendered = decoratedBinding.LowerSample(decorated);
            decoratedRendered.Descendants("SECTION").Single().Elements("P").Where(p => TaggedTemplateBinding.DirectText(p).StartsWith("{{md2hwp:", StringComparison.Ordinal)).Remove();
            decoratedRendered.Descendants("SECTION").Single().Add(Paragraph(content.Lines.Single().Text));
            var decoratedResult = decoratedBinding.Attach(decoratedRendered, plan with { Operations = new[] { extendedOperation } });
            foreach (var decoratedParagraph in decoratedResult.Descendants("FOOTNOTE").Last().Element("PARALIST")!.Elements("P").Skip(1))
            {
                Check(TaggedTemplateBinding.DirectText(decoratedParagraph) == "앞둘째 문단뒤" && decoratedParagraph.Descendants("TAB").Single().Attribute("Width")!.Value == "720",
                    "Continuation literal prefix/suffix or tab was not repeated.");
                var numbers = decoratedParagraph.Descendants("AUTONUM").ToArray();
                Check(numbers.Length == 2 && (int?)numbers[0].Attribute("Number") == 23 && (int?)numbers[1].Attribute("Number") == 7 &&
                    decoratedParagraph.Elements("TEXT").First().Elements().First().Name == "AUTONUM" && decoratedParagraph.Elements("TEXT").Last().Elements().Last().Name == "AUTONUM",
                    "Continuation automatic-number values, formatting or positions changed.");
            }
            Check(decorated.ToString() == decoratedBefore, "Decorated continuation sample mutated its source.");
            foreach (var problem in new[] { "control", "page-break", "duplicate", "extra-paragraph" })
            {
                var invalid = new XDocument(separate);
                var invalidContinuation = invalid.Descendants("P").Single(p => TaggedTemplateBinding.DirectText(p) == TaggedTemplateBinding.Tag("footnote.next"));
                if (problem == "control") invalidContinuation.Element("TEXT")!.Add(new XElement("NEWNUM"));
                if (problem == "page-break") invalidContinuation.SetAttributeValue("PageBreak", "true");
                if (problem == "duplicate") invalidContinuation.AddAfterSelf(new XElement(invalidContinuation));
                if (problem == "extra-paragraph") firstSample = invalid.Descendants("P").Single(p => TaggedTemplateBinding.DirectText(p).Trim() == TaggedTemplateBinding.Tag("footnote"));
                if (problem == "extra-paragraph") firstSample.AddAfterSelf(Paragraph("extra"));
                Reject(() => NativeFootnotes.Bind(invalid));
            }
        }
        foreach (var problem in new[] { "missing-number", "extra-paragraph", "duplicate", "table", "header", "wrong-number", "control", "tab-in-tag", "line-break", "outside" })
        {
            var invalid = new XDocument(fixture);
            var body = invalid.Descendants("P").Single(p => TaggedTemplateBinding.DirectText(p).Trim() == TaggedTemplateBinding.Tag("footnote"));
            var control = body.Ancestors("FOOTNOTE").Single();
            var host = body.Ancestors("P").Last();
            switch (problem)
            {
                case "missing-number": body.Descendants("AUTONUM").Remove(); break;
                case "extra-paragraph": body.AddAfterSelf(Paragraph("extra")); break;
                case "duplicate": host.AddAfterSelf(Paragraph(TaggedTemplateBinding.Tag("footnote"))); break;
                case "table": host.ReplaceWith(Paragraph("", new XElement("TABLE", new XElement("ROW", new XElement("CELL", new XElement("PARALIST", new XElement(host))))))); break;
                case "header": host.ReplaceWith(Paragraph("", new XElement("HEADER", new XElement("PARALIST", new XElement(host))))); break;
                case "wrong-number": control.Descendants("AUTONUM").Single().SetAttributeValue("NumberType", "Endnote"); break;
                case "control": body.Elements("TEXT").First().Add(new XElement("SECDEF")); break;
                case "tab-in-tag": body.Elements("TEXT").Skip(1).First().Element("CHAR")!.Add(new XElement("TAB")); break;
                case "line-break": body.Elements("TEXT").Skip(1).First().Element("CHAR")!.AddFirst(new XElement("LINEBREAK")); break;
                case "outside": host.Remove(); invalid.Descendants("SECTION").Single().Add(host); break;
            }
            try { NativeFootnotes.Bind(invalid); }
            catch (InvalidDataException) { continue; }
            throw new Exception("Invalid native footnote sample was accepted: " + problem);
        }
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
        try { action(); } catch (Exception error) when (error is InvalidOperationException or InvalidDataException) { return; }
        throw new Exception("Expected ambiguous or lost generated footnote rejection.");
    }
}
