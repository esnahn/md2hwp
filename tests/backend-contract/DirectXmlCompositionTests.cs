using System.Reflection;
using System.Xml.Linq;
using Md2Hwp.HancomIrPreview;

internal static class DirectXmlCompositionTests
{
    private const string NoteMarker = "MD2HWP_FOOTNOTE_11111111111111111111111111111111";
    private const string ReferenceMarker = "MD2HWP_CROSS_REFERENCE_22222222222222222222222222222222";

    internal static void Run()
    {
        StyleDefaultsAndCharacterVariants();
        UnicodeControlsAndEmptyLines();
        OperationContractsAndLiteralMarkers();
        NativeFootnoteAttachment();
        FlatCompositionPreservesTemplateAndResourceBoundaries();
        Console.WriteLine("Direct XML paragraphs preserve style defaults, Unicode, controls, rich marks and native-note markers without COM insertion.");
    }

    private static void StyleDefaultsAndCharacterVariants()
    {
        var document = Fixture();
        var styles = Bind(document);
        var originalStyles = document.Descendants("STYLE").Select(e => new XElement(e)).ToArray();
        var originalParagraphShapes = document.Descendants("PARASHAPE").Select(e => new XElement(e)).ToArray();
        var originalBodyShape = new XElement(Shape(document, "0"));
        var writer = new DirectXmlParagraphs(document, styles);
        var paragraph = writer.Create("body", [[new("plain", false, false), new("strong", true, false),
            new("emphasis", false, true), new("both", true, true), new("normal", false, false)]]);
        Check((string?)paragraph.Attribute("Style") == "0" && (string?)paragraph.Attribute("ParaShape") == "0",
            "Direct paragraphs used the tag sample's direct formatting instead of the style definition.");
        var runs = paragraph.Elements("TEXT").ToArray();
        Check(runs.Length == 5, "Distinct rich-text runs lost their independently controlled character formats.");
        var expectedMarks = new[] { (false, false), (true, false), (false, true), (true, true), (false, false) };
        for (var i = 0; i < runs.Length; i++)
        {
            var shape = Shape(document, (string)runs[i].Attribute("CharShape")!);
            Check((shape.Element("BOLD") is not null, shape.Element("ITALIC") is not null) == expectedMarks[i],
                "Direct character format lost or leaked a strong/emphasis mark.");
            var underlying = new XElement(shape); underlying.Attribute("Id")?.Remove();
            underlying.Elements("BOLD").Remove(); underlying.Elements("ITALIC").Remove();
            var baselineContent = new XElement(originalBodyShape); baselineContent.Attribute("Id")?.Remove();
            Check(XNode.DeepEquals(underlying, baselineContent),
                "A rich-text variant changed the source font, size, spacing, language settings or background.");
        }
        var afterFirst = document.Descendants("CHARSHAPE").Count();
        var repeated = writer.Create("body", [[new("another", true, true), new("same", true, false)]]);
        Check(document.Descendants("CHARSHAPE").Count() == afterFirst,
            "Repeated emphasis created duplicate character-shape definitions.");
        Check((string?)repeated.Elements("TEXT").First().Attribute("CharShape") == (string?)runs[3].Attribute("CharShape"),
            "Equal character variants did not reuse a native formatting definition.");

        var heading = writer.Create("heading1", [[new("bold baseline", false, false), new("also italic", false, true)]]);
        Check((string?)heading.Attribute("Style") == "1" && (string?)heading.Attribute("ParaShape") == "1",
            "The heading's native style or numbering paragraph shape changed.");
        Check(heading.Elements("TEXT").All(run => Shape(document, (string)run.Attribute("CharShape")!).Element("BOLD") is not null),
            "A non-strong manuscript run disabled the heading style's baseline bold format.");
        Check(Shape(document, (string)heading.Elements("TEXT").Last().Attribute("CharShape")!).Element("ITALIC") is not null,
            "Manuscript emphasis was not combined with the heading's baseline bold format.");
        var source = writer.Create("figure.source", [[new("baseline italic", false, false)]]);
        Check((string?)source.Attribute("ParaShape") == "2" &&
            Shape(document, (string)source.Element("TEXT")!.Attribute("CharShape")!).Element("ITALIC") is not null,
            "The source style's hanging-indent paragraph shape or baseline emphasis was lost.");
        Check(originalStyles.Zip(document.Descendants("STYLE")).All(pair => XNode.DeepEquals(pair.First, pair.Second)) &&
            originalParagraphShapes.Zip(document.Descendants("PARASHAPE")).All(pair => XNode.DeepEquals(pair.First, pair.Second)),
            "Paragraph composition modified existing style or paragraph-shape definitions.");
        Check(XNode.DeepEquals(Shape(document, "0"), originalBodyShape), "Character variants modified their original baseline definition.");
    }

    private static void UnicodeControlsAndEmptyLines()
    {
        var document = Fixture();
        var writer = new DirectXmlParagraphs(document, Bind(document));
        var paragraph = writer.Create("body", [[new("한글 © 😀 e\u0301  \t탭\t", false, false)], [], [new("끝 ", false, true)]]);
        Check(DisplayedText(paragraph) == "한글 © 😀 e\u0301  \t탭\t\n\n끝 ",
            "Direct XML changed Unicode, literal spaces/tabs, blank lines or trailing whitespace.");
        Check(paragraph.Descendants("TAB").Count() == 2 && paragraph.Descendants("LINEBREAK").Count() == 2,
            "Tabs or paragraph-internal line breaks were not represented by native controls.");
        Check(paragraph.Elements("TEXT").Elements("CHAR").SelectMany(character => character.Nodes().OfType<XText>())
            .All(text => !text.Value.Contains('\t') && !text.Value.Contains('\n')),
            "Text payloads retained tab/newline characters instead of native controls.");
        var empty = writer.Create("body", []);
        Check(DisplayedText(empty) == "" && (string?)empty.Attribute("Style") == "0" && (string?)empty.Attribute("ParaShape") == "0",
            "The empty terminal paragraph lost its body formatting or gained visible content.");
        Check(!empty.Descendants().Any(e => e.Name == "AUTONUM" || e.Name == "FOOTNOTE" || e.Name == "FIELDBEGIN"),
            "An empty direct paragraph inherited a generated control.");
    }

    private static void OperationContractsAndLiteralMarkers()
    {
        var document = Fixture();
        var styles = Bind(document);
        var writer = new DirectXmlParagraphs(document, styles);
        const string literal = "{{md2hwp:num:heading1}} {{md2hwp:meta:title}} {{md2hwp:begin:figure}} https://example.net";
        var note = new PreviewFootnote([PreviewInlineContent.Plain(["각주 본문"])]);
        var reference = new PreviewCrossReference("figure_number", "그림-대상");
        PreviewTextRun[] runs = [new(literal + " ", false, false), new(NoteMarker, true, false, Footnote: note),
            new(" ", false, false), new(ReferenceMarker, false, true, CrossReference: reference)];
        var operation = new PreviewOperation("text", "body", [string.Concat(runs.Select(run => run.Text))],
            ParagraphStyle: "body", FormattedLines: [runs]);
        var paragraph = writer.Create(operation).Single();
        Check(DisplayedText(paragraph) == operation.Lines.Single(), "Tag-like manuscript text or native markers were interpreted during direct composition.");
        Check(!paragraph.Descendants("FIELDBEGIN").Any() && !paragraph.Descendants("FOOTNOTE").Any() && !paragraph.Descendants("AUTONUM").Any(),
            "Direct paragraph construction prematurely converted literals or adapter markers to native controls.");
        HancomPreviewWriter.RequireNoHyperlinks([paragraph]);
        Check(operation.FormattedLines![0][1].Footnote == note && operation.FormattedLines[0][3].CrossReference == reference,
            "Direct composition mutated the IR's note or cross-reference values.");

        var plain = new PreviewOperation("text", "plain", ["first", "", "last"], ParagraphStyle: "body",
            FormattedLines: [[new("first", false, false)], [], [new("last", false, false)]]);
        Check(DisplayedText(writer.Create(plain).Single()) == "first\n\nlast", "Plain text lost preserved line breaks.");
        var table = new PreviewOperation("table", "table placeholder", ["MD2HWP_GENERATED_TABLE_literal"], ParagraphStyle: "body",
            FormattedLines: [[new("MD2HWP_GENERATED_TABLE_literal", false, false)]]);
        Check(DisplayedText(writer.Create(table).Single()) == table.Lines.Single(), "A table placeholder was not retained for native table attachment.");
        var splitDocument = Fixture();
        var preservingProfile = Profile(splitDocument);
        var splitProfile = (InvestigationTemplateProfile)typeof(InvestigationTemplateProfile)
            .GetConstructors(BindingFlags.Instance | BindingFlags.NonPublic).Single().Invoke([
                preservingProfile.Id, preservingProfile.ProfilePath, preservingProfile.TemplateSha256, preservingProfile.TemplateBytes,
                preservingProfile.ParagraphStyles, preservingProfile.ResetNativeStyle, preservingProfile.Figure, preservingProfile.Lists,
                preservingProfile.InsertionTarget, preservingProfile.BoxSelector, preservingProfile.CaptionSelector]);
        var splitStyles = AuriPreviewStyleBindings.BindDocument(splitDocument, splitProfile);
        var split = new DirectXmlParagraphs(splitDocument, splitStyles).Create(plain);
        Check(split.Select(DisplayedText).SequenceEqual(plain.Lines) && split.All(p => !p.Descendants("LINEBREAK").Any()),
            "The nonpreserving profile failed to produce one native paragraph for each IR line.");
        Reject(() => writer.Create(operation with { FormattedLines = [[new("wrong text", false, false)]] }));
        Reject(() => writer.Create(operation with { FormattedLines = [] }));
        Reject(() => writer.Create(operation with { FormattedLines = null }));
        Reject(() => writer.Create(operation with { Kind = "figure" }));
        Reject(() => writer.Create("missing role", []));
    }

    private static void NativeFootnoteAttachment()
    {
        var source = Fixture();
        var binding = NativeFootnotes.Bind(source);
        var rendered = binding.LowerSample(source);
        var section = rendered.Descendants("SECTION").Single();
        section.Elements("P").Where(p => TaggedTemplateBinding.DirectText(p).StartsWith("{{md2hwp:", StringComparison.Ordinal)).Remove();
        var writer = new DirectXmlParagraphs(rendered, Bind(rendered));
        var note = new PreviewFootnote([PreviewInlineContent.Plain(["첫째 각주"]), PreviewInlineContent.Plain(["둘째 문단"])]);
        PreviewTextRun[] runs = [new("앞 ", false, false), new(NoteMarker, true, true, Footnote: note), new(" 뒤", false, false)];
        var operation = new PreviewOperation("text", "body", [string.Concat(runs.Select(run => run.Text))], ParagraphStyle: "body", FormattedLines: [runs]);
        var generated = writer.Create(operation).Single();
        section.Add(generated, writer.Create("body", []));
        var before = rendered.ToString(SaveOptions.DisableFormatting);
        var plan = new IrPreviewPlan("fixture", "fixture", new(1, 1, 0, 0, 0), [operation], []);
        var attached = binding.Attach(rendered, plan);
        Check(rendered.ToString(SaveOptions.DisableFormatting) == before, "Native note attachment mutated the direct flat document.");
        var controls = attached.Descendants("FOOTNOTE").ToArray();
        Check(controls.Length == 1 && !attached.Descendants("CHAR").Any(character => character.Value.Contains(NoteMarker, StringComparison.Ordinal)),
            "A directly composed footnote marker was lost, duplicated or left in the output.");
        var paragraphs = controls.Single().Element("PARALIST")!.Elements("P").ToArray();
        Check(paragraphs.Length == 2 && TaggedTemplateBinding.DirectText(paragraphs[0]) == " 첫째 각주" &&
            TaggedTemplateBinding.DirectText(paragraphs[1]) == "둘째 문단", "Direct note attachment lost the native separator or continuation paragraph.");
        Check((int?)paragraphs[0].Descendants("AUTONUM").Single().Attribute("Number") == 1 && !paragraphs[1].Descendants("AUTONUM").Any(),
            "Direct notes did not preserve native numbering only on the first note paragraph.");
        Check((string?)paragraphs[0].Attribute("ParaShape") == "2" && (string?)paragraphs[1].Attribute("ParaShape") == "2",
            "Direct note insertion lost the source sample's full paragraph formatting.");
        var host = controls.Single().Ancestors("P").Last();
        var numberShape = Shape(attached, (string)controls.Single().Parent!.Attribute("CharShape")!);
        Check(numberShape.Element("BOLD") is not null && numberShape.Element("ITALIC") is not null &&
            TaggedTemplateBinding.DirectText(host) == "앞  뒤", "The direct footnote callout lost its source emphasis or surrounding body text.");
        Check(XNode.DeepEquals(source.Descendants("FOOTNOTESHAPE").Single(), attached.Descendants("FOOTNOTESHAPE").Single()),
            "Direct note composition changed section-owned numbering or placement options.");
    }

    private static void FlatCompositionPreservesTemplateAndResourceBoundaries()
    {
        var template = Fixture();
        static XElement P(string text, string style = "0", string paragraph = "0", string character = "0") =>
            new("P", new XAttribute("Style", style), new XAttribute("ParaShape", paragraph),
                new XElement("TEXT", new XAttribute("CharShape", character), new XElement("CHAR", text)));
        var section = template.Descendants("SECTION").Single();
        var cover = new XElement(section.Elements("P").First());
        cover.Descendants("SECDEF").Single().Add(new XElement("MASTERPAGE",
            new XElement("LINE", new XElement("SHAPEOBJECT", new XAttribute("ZOrder", "40")))));
        cover.Element("TEXT")!.Add(new XElement("FIELDBEGIN", new XAttribute("Type", "Hyperlink"), new XAttribute("FieldId", "91"), new XAttribute("InstId", "92")),
            new XElement("CHAR", "existing template link"), new XElement("FIELDEND", new XAttribute("Type", "Hyperlink"), new XAttribute("FieldId", "91")));
        var codeSource = P("출처: {{md2hwp:slot:code.source}}", "3", "2", "3");
        var figureSource = P("출처: {{md2hwp:slot:figure.source}}", "3", "2", "3");
        var code = P("");
        code.Element("TEXT")!.ReplaceNodes(new XElement("TABLE",
            new XElement("SHAPEOBJECT", new XAttribute("InstId", "code-prototype"),
                new XElement("SIZE", new XAttribute("Width", "10000"), new XAttribute("Height", "1000")),
                new XElement("POSITION", new XAttribute("TreatAsChar", "true")),
                new XElement("CAPTION", new XAttribute("Side", "Bottom"), new XElement("PARALIST", codeSource))),
            new XElement("ROW", new XElement("CELL", new XAttribute("Width", "10000"),
                new XElement("PARALIST", P("{{md2hwp:slot:code.content}}"))))), new XElement("CHAR", ""));
        var caption = P("[그림 ");
        caption.Element("TEXT")!.Add(new XElement("AUTONUM", new XAttribute("NumberType", "Figure"), new XAttribute("Number", "1"),
            new XElement("AUTONUMFORMAT", new XAttribute("Type", "Digit"))), new XElement("CHAR", "] {{md2hwp:slot:figure.caption}}"));
        section.ReplaceNodes(cover, P(TaggedTemplateBinding.Tag("begin:template")), code, caption, figureSource,
            P(TaggedTemplateBinding.Tag("end:template")), P(TaggedTemplateBinding.Tag("content")), P(""));
        var profile = InvestigationTemplateProfile.FromTaggedTemplate(typeof(IrPreviewPlan).Assembly.Location,
            [new("body", "Body"), new("heading1", "Heading"), new("code.anchor", "Body"), new("code", "Body"),
                new("code.source", "Source"), new("figure", "Body"), new("figure.caption", "Body"), new("figure.source", "Source")],
            "Reset", 142, 6, 380, new("figure.caption", "[그림 ", "] {{md2hwp:slot:figure.caption}}", TaggedTemplateBinding.Tag("slot:figure.caption")),
            new(new XElement(codeSource), TaggedTemplateBinding.Tag("slot:code.source"), "출처: ", ""),
            new(new XElement(figureSource), TaggedTemplateBinding.Tag("slot:figure.source"), "출처: ", ""),
            new("bullet", 0, new XElement("BULLET")), new("ordered", 0, new XElement("NUMBERING")));
        var binding = new TaggedTemplateBinding(profile, 1, 5, 6, 2, 3);
        var styles = AuriPreviewStyleBindings.BindDocument(template, profile);
        var resources = new XDocument(template);
        resources.Descendants("SECTION").Single().Elements("P").First().Elements("TEXT").First().Element("CHAR")!.Value = "resource cover must not leak";
        var binary = new XElement("BINDATA", new XAttribute("Id", "7"), new XAttribute("Encoding", "Base64"), "AQID");
        resources.Root!.Add(new XElement("TAIL", new XElement("BINDATASTORAGE", binary)));
        XElement Picture(string id) => new("PICTURE", new XElement("SHAPEOBJECT", new XAttribute("InstId", id),
            new XElement("SIZE", new XAttribute("Width", "3210"), new XAttribute("Height", "1234"))),
            new XElement("IMAGE", new XAttribute("BinItem", "7")));
        var pictures = new[] { Picture("picture-a"), Picture("picture-b") };
        const string literal = "원고 {{md2hwp:num:heading1}} {{md2hwp:meta:title}} https://example.net";
        PreviewOperation Figure(string text, string sourceText) => new("figure", text, ["alt", text, sourceText],
            ParagraphStyle: "figure", FormattedLines: [[new("alt", false, false)], [new(text, true, false)],
                sourceText.Length == 0 ? [] : [new(sourceText, true, false)]]);
        PreviewOperation[] operations = [new("text", "body", [literal], ParagraphStyle: "body", FormattedLines: [[new(literal, false, false)]]),
            new("code", "with source", ["  첫줄\t", "", "마지막"], ParagraphStyle: "code", SourceRuns: [new("법령", true, false)]),
            Figure("그림 설명", "그림 자료"), new("code", "without source", ["출처: 원문 그대로"], ParagraphStyle: "code"), Figure("둘째 그림", "")];
        var plan = new IrPreviewPlan("fixture", "fixture", new(5, 1, 2, 2, 0), operations, []);
        var templateBefore = template.ToString(SaveOptions.DisableFormatting);
        var resourcesBefore = resources.ToString(SaveOptions.DisableFormatting);
        var pictureBefore = pictures.Select(p => p.ToString(SaveOptions.DisableFormatting)).ToArray();
        var composed = DirectXmlFlatDocument.Compose(template, resources, binding, styles, plan, pictures);
        var roots = AuriMinimalBoxPrototype.RootParagraphs(composed.Document);
        Check(composed.Start == 1 && roots.Count == 10, "Direct flat composition changed static/generated root boundaries.");
        Check(XNode.DeepEquals(roots[0], cover) && XNode.DeepEquals(composed.Document.Descendants("SECDEF").Single(), cover.Descendants("SECDEF").Single()),
            "Resource collection displaced static template content or native section settings.");
        Check(composed.Document.Descendants("FIELDBEGIN").Count() == 1 && (string?)composed.Document.Descendants("FIELDBEGIN").Single().Attribute("Type") == "Hyperlink",
            "An unrelated existing template hyperlink was removed or generated hyperlinks were added.");
        HancomPreviewWriter.RequireNoHyperlinks(roots.Skip(composed.Start));
        Check(DisplayedText(roots[1]) == literal, "Direct flat composition recursively interpreted manuscript literals.");
        var boxes = roots.SelectMany(root => root.Descendants("TABLE")).ToArray();
        Check(boxes.Length == 2 && DisplayedText(boxes[0].Descendants("CELL").Single().Descendants("P").Single()) == "  첫줄\t\n\n마지막",
            "Direct code composition changed verbatim whitespace or empty lines before paragraph expansion.");
        Check(DisplayedText(boxes[0].Descendants("CAPTION").Single().Descendants("P").Single()) == "출처: 법령" && !boxes[1].Descendants("CAPTION").Any(),
            "Direct code composition lost the source prefix or retained an absent source caption.");
        Check(composed.Document.Descendants("PICTURE").Count() == 2 && composed.Document.Descendants("AUTONUM").Count() == 2,
            "Direct figure composition lost native picture resources or native figure numbering.");
        Check(composed.Document.Descendants("PICTURE").Select(e => e.ToString(SaveOptions.DisableFormatting)).SequenceEqual(pictureBefore),
            "Direct composition changed live-created picture payloads, dimensions or resource IDs.");
        Check(XNode.DeepEquals(binary, composed.Document.Descendants("BINDATA").Single()), "Direct composition changed embedded picture data.");
        Check(DisplayedText(roots[4]) == "[그림 ] 그림 설명" && DisplayedText(roots[5]) == "출처: 그림 자료" && DisplayedText(roots[8]) == "[그림 ] 둘째 그림",
            "Direct figures lost their template affixes, sources or source omission boundaries.");
        Check((string?)roots[4].Elements("TEXT").Last().Attribute("CharShape") == "0" &&
            roots[4].Elements("TEXT").Last().Element("CHAR")!.Value == "",
            "A marked caption ending lost the native slot's empty editing-format run.");
        var ordered = DirectXmlFlatDocument.Compose(template, resources, binding, styles, plan, pictures, zOrderStart: 12);
        Check(AuriMinimalBoxPrototype.RootParagraphs(ordered.Document)
            .SelectMany(root => root.Elements("TEXT").Elements().Where(e => e.Name == "TABLE" || e.Name == "PICTURE"))
            .Select(e => (int)e.Element("SHAPEOBJECT")!.Attribute("ZOrder")!).SequenceEqual(new[] { 12, 13, 14, 15 }),
            "Interleaved code and figure objects did not retain native insertion order from the supplied allocator seed.");
        Check((int?)ordered.Document.Descendants("MASTERPAGE").Single().Descendants("SHAPEOBJECT").Single().Attribute("ZOrder") == 42,
            "Native master-page allocation did not reserve the additional generated code objects.");
        var renumbered = new XDocument(resources);
        foreach (var shape in renumbered.Descendants("CHARSHAPE"))
            shape.SetAttributeValue("Id", (int)shape.Attribute("Id")! + 100);
        foreach (var reference in renumbered.Descendants().Attributes("CharShape"))
            reference.Value = ((int)reference + 100).ToString(System.Globalization.CultureInfo.InvariantCulture);
        var remapped = DirectXmlFlatDocument.Compose(template, renumbered, binding, styles, plan, pictures);
        Check(TemplateRangeStructure.Equivalent(roots, AuriMinimalBoxPrototype.RootParagraphs(remapped.Document), composed.Document, remapped.Document),
            "Native resource format renumbering changed static content, code, captions or generated paragraph formats.");
        Check((string?)roots[^1].Attribute("Style") == "0" && DisplayedText(roots[^1]) == "" &&
            HancomPreviewWriter.ReadParagraphs(composed.Document)[^1].NativeList is null, "The terminal paragraph retained a heading, list or visible content.");
        Check(template.ToString(SaveOptions.DisableFormatting) == templateBefore && resources.ToString(SaveOptions.DisableFormatting) == resourcesBefore &&
            pictures.Select(p => p.ToString(SaveOptions.DisableFormatting)).SequenceEqual(pictureBefore),
            "Direct flat composition mutated its template, live resource snapshot or picture inputs.");
        Reject(() => DirectXmlFlatDocument.Compose(template, resources, binding, styles, plan, pictures.Take(1).ToArray()));
    }

    private static XDocument Fixture() => XDocument.Parse("""
        <HWPML><HEAD>
        <CHARSHAPELIST Count="4">
        <CHARSHAPE Id="0" Height="1000" BorderFillId="0" UseFontSpace="true"><FONTID Hangul="0" Latin="1"/><RATIO Hangul="95" Latin="98"/><CHARSPACING Hangul="-3" Latin="1"/><RELSIZE Hangul="100" Latin="90"/><COLOR Value="4278190080"/></CHARSHAPE>
        <CHARSHAPE Id="1" Height="1600"><BOLD/></CHARSHAPE>
        <CHARSHAPE Id="2" Height="1100"/>
        <CHARSHAPE Id="3" Height="900"><ITALIC/></CHARSHAPE>
        </CHARSHAPELIST>
        <PARASHAPELIST Count="4">
        <PARASHAPE Id="0" HeadingType="None"><PARAMARGIN Left="120" Right="80" Indent="0"/></PARASHAPE>
        <PARASHAPE Id="1" HeadingType="Number" Heading="0" Level="0"><PARAMARGIN Left="700" Right="0" Indent="-180"/></PARASHAPE>
        <PARASHAPE Id="2" HeadingType="None"><PARAMARGIN Left="1300" Right="0" Indent="-400"/></PARASHAPE>
        <PARASHAPE Id="3" HeadingType="None"><PARAMARGIN Left="4700" Right="0" Indent="500"/></PARASHAPE>
        </PARASHAPELIST>
        <NUMBERINGLIST Count="1"><NUMBERING Id="0" Start="1"><PARAHEAD Level="1" Start="1" CharShape="1" NumFormat="Digit">^1.</PARAHEAD></NUMBERING></NUMBERINGLIST>
        <STYLELIST Count="4">
        <STYLE Id="0" Type="Para" Name="Body" CharShape="0" ParaShape="0"/>
        <STYLE Id="1" Type="Para" Name="Heading" CharShape="1" ParaShape="1"/>
        <STYLE Id="2" Type="Para" Name="Reset" CharShape="2" ParaShape="0"/>
        <STYLE Id="3" Type="Para" Name="Source" CharShape="3" ParaShape="2"/>
        </STYLELIST></HEAD>
        <DOCSETTING><BEGINNUMBER Footnote="1"/></DOCSETTING>
        <BODY><SECTION>
        <P Style="0" ParaShape="0"><TEXT CharShape="0"><SECDEF><FOOTNOTESHAPE><AUTONUMFORMAT Type="Digit" SuffixChar=")"/><NOTENUMBERING Type="Continuous" NewNumber="1"/></FOOTNOTESHAPE></SECDEF><CHAR>static cover</CHAR></TEXT></P>
        <P Style="0" ParaShape="0"><TEXT CharShape="0"><CHAR>{{md2hwp:begin:template}}</CHAR></TEXT></P>
        <P Style="0" ParaShape="3"><TEXT CharShape="3"><CHAR>{{md2hwp:body}}</CHAR></TEXT></P>
        <P Style="3" ParaShape="2"><TEXT CharShape="3"><CHAR>{{md2hwp:footnote}}</CHAR></TEXT></P>
        <P Style="0" ParaShape="0"><TEXT CharShape="0"><CHAR>{{md2hwp:end:template}}</CHAR></TEXT></P>
        </SECTION></BODY></HWPML>
        """);

    private static AuriPreviewStyleBindings Bind(XDocument document) => AuriPreviewStyleBindings.BindDocument(document, Profile(document));
    private static InvestigationTemplateProfile Profile(XDocument document)
    {
        var sample = document.Descendants("SECTION").Single().Elements("P").First();
        var source = new TemplateSource(new XElement(sample), "source", "", "");
        return InvestigationTemplateProfile.FromTaggedTemplate(typeof(IrPreviewPlan).Assembly.Location,
            [new("body", "Body"), new("heading1", "Heading"), new("figure.source", "Source")],
            "Reset", 142, 6, 380, new("body", "[", "]slot", "slot"), source, source,
            new("bullet", 0, new XElement("BULLET")), new("ordered", 0, new XElement("NUMBERING")));
    }
    private static XElement Shape(XDocument document, string id) => document.Descendants("CHARSHAPE").Single(shape => (string?)shape.Attribute("Id") == id);
    private static string DisplayedText(XElement paragraph) => string.Concat(paragraph.Elements("TEXT").SelectMany(run => run.Elements("CHAR")).SelectMany(character => character.Nodes()).Select(node => node switch
    {
        XText text => text.Value,
        XElement control when control.Name == "TAB" => "\t",
        XElement control when control.Name == "LINEBREAK" => "\n",
        _ => ""
    }));
    private static void Check(bool value, string message) { if (!value) throw new Exception(message); }
    private static void Reject(Action action) { try { action(); } catch (Exception error) when (error is InvalidOperationException or InvalidDataException) { return; } throw new Exception("Expected direct-XML contract rejection."); }
}
