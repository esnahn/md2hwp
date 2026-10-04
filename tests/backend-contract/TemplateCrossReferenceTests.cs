using System.Xml.Linq;
using Md2Hwp.HancomIrPreview;

internal static class TemplateCrossReferenceTests
{
    private static readonly string[] Roles = [TemplateCrossReferences.FigureNumberRole, TemplateCrossReferences.HeadingNumberRole];

    internal static void Run()
    {
        var original = Fixture();
        var originalText = original.ToString();
        var lowered = TemplateCrossReferences.Lower(original);
        Check(original.ToString() == originalText, "Reference lowering mutated its source.");
        Check(AuriMinimalBoxPrototype.RootParagraphs(lowered.Document).Select(TaggedTemplateBinding.DirectText)
            .SequenceEqual(new[] { "before", Tag("begin:template"), "other declaration", Tag("end:template"), "after" }),
            "Reference lowering did not remove exactly the six prototype roots.");

        var destination = Destination();
        var figure = lowered.Layout.CreateFigureNumberFragments(destination, 7, "NATIVE_FIGURE_7");
        Check(Text(figure) == "[그림 7-NATIVE_FIGURE_7]", "Figure reference did not use its target chapter.");
        Check(figure.All(fragment => fragment.Name == "TEXT") && !new XElement("P", figure).Attributes().Any(),
            "Reference paragraph formatting was applied to inline fragments.");
        var figureRun = figure.Single(fragment => fragment.Value.Contains("NATIVE_FIGURE_7", StringComparison.Ordinal));
        Check((string?)figureRun.Attribute("CharShape") == "9" && (string?)figureRun.Attribute("RunProperty") == "figure-number",
            "Figure number lost its first slot character's complete run formatting.");
        var chapterRun = figure.Single(fragment => fragment.Value.Contains("7", StringComparison.Ordinal) &&
            !fragment.Value.Contains("NATIVE_FIGURE_7", StringComparison.Ordinal));
        var chapterShape = destination.Descendants("CHARSHAPE").Single(shape =>
            (string?)shape.Attribute("Id") == (string?)chapterRun.Attribute("CharShape"));
        Check((string?)chapterRun.Attribute("RunProperty") == "prefix" &&
            (string?)chapterShape.Attribute("Height") == "1000" && (string?)chapterShape.Attribute("TextColor") == "0",
            "Chapter number lost the first tag character's source formatting.");
        Check(destination.Descendants("CHARSHAPE").Count() == 4 &&
            destination.Descendants("CHARSHAPE").Any(shape => shape.Element("BOLD") is not null),
            "Reference formatting import lost or duplicated a character shape.");

        var heading = lowered.Layout.CreateHeadingNumberFragments(destination, "NATIVE_OUTLINE");
        Check(Text(heading) == "제NATIVE_OUTLINE절", "Heading reference altered its native outline slot or literal wrapper.");
        var headingRun = heading.Single(fragment => fragment.Value.Contains("NATIVE_OUTLINE", StringComparison.Ordinal));
        Check((string?)headingRun.Attribute("CharShape") == "9" && (string?)headingRun.Attribute("RunProperty") == "heading-number",
            "Heading native outline number lost its first slot character's formatting.");
        Check(destination.Descendants("CHARSHAPE").Count() == 4, "Heading reference duplicated equivalent character formatting.");
        Check(Text(lowered.Layout.CreateFigureNumberFragments(destination, 8, "NATIVE_FIGURE_8")) == "[그림 8-NATIVE_FIGURE_8]",
            "A reference instance retained its previous target chapter.");
        Check(original.ToString() == originalText, "Reference instantiation mutated a source prototype.");

        var prepared = TemplateCrossReferences.Lower(TemplateHeadingNumbers.Prepare(original));
        Check(Text(prepared.Layout.CreateFigureNumberFragments(Destination(), 11, "NATIVE_FIGURE_PREPARED")) == "[그림 11-NATIVE_FIGURE_PREPARED]",
            "Prepared figure chapter slots did not use the target context.");
        Check(Text(prepared.Layout.CreateHeadingNumberFragments(Destination(), "NATIVE_OUTLINE_PREPARED")) == "제NATIVE_OUTLINE_PREPARED절",
            "Heading reference required or calculated a chapter number.");
        Reject(() => prepared.Layout.CreateFigureNumberFragments(Destination(), null, "NATIVE_NO_CHAPTER"), "heading1");
        Reject(() => lowered.Layout.CreateFigureNumberFragments(Destination(), null, "NATIVE_NO_CHAPTER"), "target figure");
        var withoutChapter = TemplateCrossReferences.Lower(Fixture(Paragraph("See " + TemplateCrossReferences.FigureNumberSlot)));
        Check(Text(withoutChapter.Layout.CreateFigureNumberFragments(Destination(), null, "NATIVE_NO_CHAPTER")) == "See NATIVE_NO_CHAPTER",
            "A figure prototype without a chapter slot required a chapter.");

        foreach (var role in Roles)
        {
            var missing = Fixture();
            RemovePrototype(missing, role);
            Reject(() => TemplateCrossReferences.Lower(missing), "init-template");
            var duplicate = Fixture();
            Section(duplicate).Add(Paragraph(Tag("begin:" + role)));
            Reject(() => TemplateCrossReferences.Lower(duplicate), "found 2");
            var twoSamples = Fixture();
            Find(twoSamples, "begin:" + role).AddAfterSelf(Paragraph("extra"));
            Reject(() => TemplateCrossReferences.Lower(twoSamples), "exactly one root sample");
            var outside = Fixture();
            var outsideRange = Range(outside, role).Select(paragraph => new XElement(paragraph)).ToArray();
            RemovePrototype(outside, role);
            Find(outside, "end:template").AddAfterSelf(outsideRange);
            Reject(() => TemplateCrossReferences.Lower(outside), "inside begin:template");
            var reversed = Fixture();
            var begin = Find(reversed, "begin:" + role);
            var end = Find(reversed, "end:" + role);
            begin.Elements("TEXT").Elements("CHAR").Single().Value = Tag("end:" + role);
            end.Elements("TEXT").Elements("CHAR").Single().Value = Tag("begin:" + role);
            Reject(() => TemplateCrossReferences.Lower(reversed), "exactly one root sample");
            var splitBoundary = Fixture();
            Find(splitBoundary, "begin:" + role).ReplaceNodes(
                Run("0", "{{md2hwp:begin:ref."), Run("1", role["ref.".Length..] + "}}"));
            Check(TemplateCrossReferences.Lower(splitBoundary).Layout is not null, "Split-run reference boundary was rejected.");

            var slot = Tag("slot:" + role);
            foreach (var sampleText in new[]
            {
                "no slot", slot + slot, Tag("slot:figure.number"), Tag("body") + slot,
                "{{md2hwp:slot:" + role + slot,
                Tag("slot:" + (role == TemplateCrossReferences.FigureNumberRole ? TemplateCrossReferences.HeadingNumberRole : TemplateCrossReferences.FigureNumberRole)),
            })
                Reject(() => TemplateCrossReferences.Lower(WithSample(role, Paragraph(sampleText))));
            var orphan = Fixture();
            Find(orphan, "end:template").AddAfterSelf(Paragraph(slot));
            Reject(() => TemplateCrossReferences.Lower(orphan), "single root prototypes");
            foreach (var native in new[] { "AUTONUM", "SECDEF", "COLDEF", "TABLE", "PICTURE", "FOOTNOTE", "LINE", "LINEBREAK", "TAB" })
            {
                var sample = Paragraph(slot);
                sample.Element("TEXT")!.Add(new XElement(native));
                Reject(() => TemplateCrossReferences.Lower(WithSample(role, sample)), "plain");
                var boundary = Fixture();
                Find(boundary, "begin:" + role).Element("TEXT")!.Add(new XElement(native));
                Reject(() => TemplateCrossReferences.Lower(boundary), "plain");
            }
            var nested = Paragraph(slot);
            nested.Element("TEXT")!.Add(new XElement("CHAR", Paragraph("nested")));
            Reject(() => TemplateCrossReferences.Lower(WithSample(role, nested)), "plain");
            var rawText = Paragraph(slot);
            rawText.Element("TEXT")!.Add(new XText("unwrapped"));
            Reject(() => TemplateCrossReferences.Lower(WithSample(role, rawText)), "plain");
            foreach (var breakName in new[] { "PageBreak", "ColumnBreak" })
            {
                var sample = Paragraph(slot);
                sample.SetAttributeValue(breakName, "true");
                Reject(() => TemplateCrossReferences.Lower(WithSample(role, sample)), "breaks");
            }
            foreach (var control in new[] { "\n", "\r", "\t" })
                Reject(() => TemplateCrossReferences.Lower(WithSample(role, Paragraph(slot + control))), "single-line");
        }

        var headingWithChapter = Fixture(heading: Paragraph(TemplateHeadingNumbers.Tag + TemplateCrossReferences.HeadingNumberSlot));
        Reject(() => TemplateCrossReferences.Lower(headingWithChapter), "native outline");
        Reject(() => TemplateCrossReferences.Lower(TemplateHeadingNumbers.Prepare(headingWithChapter)), "native outline");
        foreach (var reserved in new[]
        {
            "ref.table.number", "ref.equation.number", "ref.footnote.number", "ref.endnote.number",
            "ref.bookmark.text", "ref.figure.page", "ref.table.page", "ref.heading.page", "ref.unknown.number",
        })
        foreach (var kind in new[] { "begin:", "end:", "slot:", "" })
        {
            var unsupported = Fixture();
            Find(unsupported, "begin:template").AddAfterSelf(Paragraph(Tag(kind + reserved)));
            Reject(() => TemplateCrossReferences.Lower(unsupported), "Unsupported template cross-reference");
        }
        foreach (var marker in new[] { "", "BAD\nMARKER", Tag("slot:any") })
        {
            Reject(() => lowered.Layout.CreateFigureNumberFragments(Destination(), 1, marker), "native control marker");
            Reject(() => lowered.Layout.CreateHeadingNumberFragments(Destination(), marker), "native control marker");
        }
        var collision = Destination();
        Section(collision).Add(Paragraph("NATIVE_COLLISION"));
        Reject(() => lowered.Layout.CreateFigureNumberFragments(collision, 1, "NATIVE_COLLISION"), "unique");
        Reject(() => lowered.Layout.CreateHeadingNumberFragments(collision, "NATIVE_COLLISION"), "unique");
        CheckGeneratedHeadingCandidates();
        Console.WriteLine("Figure/heading reference prototypes, target chapter and inline formatting checks passed.");
    }

    private static void CheckGeneratedHeadingCandidates()
    {
        var source = Fixture();
        var simple = Destination();
        var operations = Enumerable.Range(1, 6).Select(level => new PreviewOperation("text", "title", ["제목"],
            ParagraphStyle: $"heading{level}", HeadingId: $"제목{level}")).ToArray();
        foreach (var level in Enumerable.Range(1, 6))
        {
            var paragraph = Paragraph("rendered");
            paragraph.SetAttributeValue("InstId", 100 + level);
            Section(simple).Add(paragraph);
        }
        var plan = new IrPreviewPlan("fixture", "fixture", new(6, 6, 0, 0, 0), operations, []);
        var simpleLayout = new TemplateHeadingBlocks(source, new Dictionary<string, XElement[]>(StringComparer.Ordinal));
        simpleLayout.Attach(simple, plan, 0);
        Check(simpleLayout.GeneratedHeadingInstances.Count == 6 &&
            Enumerable.Range(1, 6).All(level => simpleLayout.GeneratedHeadingInstances[$"제목{level}"].Single() == (100 + level).ToString()),
            "Native identities for ordinary heading1 through heading6 were lost.");

        XElement HeadingParagraph(string value, string instance)
        {
            var paragraph = Paragraph(value);
            paragraph.SetAttributeValue("InstId", instance);
            return paragraph;
        }
        var blockLayout = new TemplateHeadingBlocks(source, new Dictionary<string, XElement[]>(StringComparer.Ordinal)
        {
            ["heading1"] =
            [
                HeadingParagraph(Tag("slot:heading1"), "201"),
                HeadingParagraph(Tag("begin:each.child:heading2"), "202"),
                HeadingParagraph(Tag("slot:heading2"), "203"),
                HeadingParagraph(Tag("end:each.child:heading2"), "204"),
            ],
        });
        var rendered = Destination();
        Section(rendered).Add(HeadingParagraph("parent", "101"), HeadingParagraph("child", "102"));
        var repeatedPlan = new IrPreviewPlan("fixture", "fixture", new(2, 2, 0, 0, 0),
            [new("text", "parent", ["parent"], ParagraphStyle: "heading1", HeadingId: "부모"),
             new("text", "child", ["child"], ParagraphStyle: "heading2", HeadingId: "자식")], []);
        var attached = blockLayout.Attach(rendered, repeatedPlan, 0);
        var parentIds = blockLayout.GeneratedHeadingInstances["부모"];
        var childIds = blockLayout.GeneratedHeadingInstances["자식"];
        Check(parentIds.Count == 1 && childIds.Count == 2 && childIds.Contains("102") &&
            childIds.Distinct(StringComparer.Ordinal).Count() == 2,
            "Repeated heading copies were discarded instead of retaining native ambiguity candidates.");
        Check(parentIds.Concat(childIds).All(id => attached.Descendants("P").Count(paragraph => (string?)paragraph.Attribute("InstId") == id) == 1),
            "Heading target mapping captured identities before final clone reassignment.");

        var repeatedSlots = new TemplateHeadingBlocks(source, new Dictionary<string, XElement[]>(StringComparer.Ordinal)
        {
            ["heading2"] = [HeadingParagraph(Tag("slot:heading2"), "301"), HeadingParagraph(Tag("slot:heading2"), "302")],
        });
        var duplicatedRendered = Destination();
        Section(duplicatedRendered).Add(HeadingParagraph("title", "103"));
        var duplicatedPlan = new IrPreviewPlan("fixture", "fixture", new(1, 1, 0, 0, 0),
            [new("text", "title", ["title"], ParagraphStyle: "heading2", HeadingId: "여러-제목-슬롯")], []);
        var duplicatedAttached = repeatedSlots.Attach(duplicatedRendered, duplicatedPlan, 0);
        var duplicatedIds = repeatedSlots.GeneratedHeadingInstances["여러-제목-슬롯"];
        Check(duplicatedIds.Count == 2 && duplicatedIds.Distinct(StringComparer.Ordinal).Count() == 2 &&
            duplicatedIds.All(id => duplicatedAttached.Descendants("P").Count(paragraph => (string?)paragraph.Attribute("InstId") == id) == 1),
            "Multiple title slots were silently reduced to one native heading target.");

        var nestedSlot = HeadingParagraph(Tag("slot:heading3"), "401");
        nestedSlot.Attribute("InstId")!.Remove();
        nestedSlot.SetAttributeValue("InstID", "401");
        var nestedRoot = Paragraph("");
        nestedRoot.Element("TEXT")!.Add(new XElement("TABLE", new XElement("ROW",
            new XElement("CELL", new XElement("PARALIST", nestedSlot)))));
        var nestedLayout = new TemplateHeadingBlocks(source, new Dictionary<string, XElement[]>(StringComparer.Ordinal)
        {
            ["heading3"] = [nestedRoot],
        });
        var nestedRendered = Destination();
        Section(nestedRendered).Add(HeadingParagraph("nested", "104"));
        var nestedPlan = new IrPreviewPlan("fixture", "fixture", new(1, 1, 0, 0, 0),
            [new("text", "nested", ["nested"], ParagraphStyle: "heading3", HeadingId: "표-안-제목")], []);
        var nestedAttached = nestedLayout.Attach(nestedRendered, nestedPlan, 0);
        var nestedId = nestedLayout.GeneratedHeadingInstances["표-안-제목"].Single();
        Check(nestedAttached.Descendants("CELL").Descendants("P").Single().Attribute("InstID")!.Value == nestedId,
            "A nested title slot or the InstID paragraph identity spelling was lost.");

        var noIdsPlan = new IrPreviewPlan("fixture", "fixture", new(6, 6, 0, 0, 0),
            operations.Select(operation => operation with { HeadingId = null }).ToArray(), []);
        simpleLayout.Attach(simple, noIdsPlan, 0);
        Check(simpleLayout.GeneratedHeadingInstances.Count == 0, "A reused heading layout retained target IDs from a prior document.");
    }
    private static string Tag(string name) => TaggedTemplateBinding.Tag(name);
    private static string Text(IEnumerable<XElement> fragments) => TaggedTemplateBinding.DirectText(new XElement("P", fragments));
    private static XElement Run(string shape, string text, string? property = null) =>
        new("TEXT", new XAttribute("CharShape", shape),
            property is null ? null : new XAttribute("RunProperty", property), new XElement("CHAR", text));
    private static XElement Paragraph(string text) => new("P", new XAttribute("ParaShape", "0"), new XAttribute("Style", "0"), Run("0", text));
    private static XElement FigureSample() => new("P", new XAttribute("ParaShape", "0"), new XAttribute("Style", "0"),
        Run("0", "[그림 {{md2hwp:num:head", "prefix"), Run("1", "ing1}}-", "chapter-continuation"),
        Run("2", "{{md2hwp:slot:ref.fi", "figure-number"), Run("1", "gure.number}}]", "number-continuation"));
    private static XElement HeadingSample() => new("P", new XAttribute("ParaShape", "0"), new XAttribute("Style", "0"),
        Run("2", "제{{md2hwp:slot:ref.head", "heading-number"), Run("1", "ing.number}}절", "number-continuation"));
    private static XDocument WithSample(string role, XElement sample) =>
        role == TemplateCrossReferences.FigureNumberRole ? Fixture(sample) : Fixture(heading: sample);
    private static XDocument Fixture(XElement? figure = null, XElement? heading = null) => new(new XElement("HWPML",
        new XElement("HEAD",
            new XElement("PARASHAPELIST", new XAttribute("Count", "1"), new XElement("PARASHAPE", new XAttribute("Id", "0"), new XAttribute("Align", "Center"))),
            new XElement("CHARSHAPELIST", new XAttribute("Count", "3"),
                new XElement("CHARSHAPE", new XAttribute("Id", "0"), new XAttribute("Height", "1000"), new XAttribute("TextColor", "0")),
                new XElement("CHARSHAPE", new XAttribute("Id", "1"), new XAttribute("Height", "1200"), new XElement("BOLD")),
                new XElement("CHARSHAPE", new XAttribute("Id", "2"), new XAttribute("Height", "1100"), new XElement("ITALIC")))),
        new XElement("BODY", new XElement("SECTION", Paragraph("before"), Paragraph(Tag("begin:template")),
            Paragraph("other declaration"), Paragraph(Tag("begin:" + TemplateCrossReferences.FigureNumberRole)), figure ?? FigureSample(),
            Paragraph(Tag("end:" + TemplateCrossReferences.FigureNumberRole)),
            Paragraph(Tag("begin:" + TemplateCrossReferences.HeadingNumberRole)), heading ?? HeadingSample(),
            Paragraph(Tag("end:" + TemplateCrossReferences.HeadingNumberRole)), Paragraph(Tag("end:template")), Paragraph("after")))));
    private static XDocument Destination() => new(new XElement("HWPML",
        new XElement("HEAD",
            new XElement("PARASHAPELIST", new XAttribute("Count", "1"), new XElement("PARASHAPE", new XAttribute("Id", "0"), new XAttribute("Align", "Left"))),
            new XElement("CHARSHAPELIST", new XAttribute("Count", "2"),
                new XElement("CHARSHAPE", new XAttribute("Id", "0"), new XAttribute("Height", "800")),
                new XElement("CHARSHAPE", new XAttribute("Id", "9"), new XAttribute("Height", "1100"), new XElement("ITALIC")))),
        new XElement("BODY", new XElement("SECTION"))));
    private static XElement Section(XDocument document) => document.Descendants("SECTION").Single();
    private static XElement Find(XDocument document, string tag) => Section(document).Elements("P")
        .First(paragraph => TaggedTemplateBinding.DirectText(paragraph) == Tag(tag));
    private static XElement[] Range(XDocument document, string role)
    {
        var roots = Section(document).Elements("P").ToArray();
        return roots.Skip(Array.IndexOf(roots, Find(document, "begin:" + role))).Take(3).ToArray();
    }
    private static void RemovePrototype(XDocument document, string role) => Range(document, role).Remove();
    private static void Check(bool condition, string message)
    {
        if (!condition) throw new Exception(message);
    }
    private static void Reject(Action action, string? expectedMessage = null)
    {
        try { action(); }
        catch (InvalidDataException error)
        {
            if (expectedMessage is not null && !error.Message.Contains(expectedMessage, StringComparison.Ordinal))
                throw new Exception($"Expected '{expectedMessage}' in rejection, got '{error.Message}'.", error);
            return;
        }
        throw new Exception("Expected template reference rejection.");
    }
}
