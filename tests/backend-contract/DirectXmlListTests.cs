using System.Xml.Linq;
using Md2Hwp.HancomIrPreview;

internal static class DirectXmlListTests
{
    internal static void Run()
    {
        OrderedSegmentsRetainStartsAndLevels();
        ContinuationsRetainParentIdentityAndFormatting();
        MarkerBackgroundsAndCanonicalAttributes();
        Console.WriteLine("Direct XML lists retain native formats, nested/restarted counters, continuation alignment and immutable template definitions.");
    }

    private static void OrderedSegmentsRetainStartsAndLevels()
    {
        var document = Fixture();
        var layout = Layout(document);
        var original = new XElement(layout.Ordered!.Definition);
        var lists = new DirectXmlLists(document, Plan(), layout);
        var first = Paragraph();
        var start = new PreviewListMarker(1, "ordered", 2, 7, 7, true);
        lists.Apply(first, start, 30);
        var firstShape = Shape(document, first);
        var firstDefinition = Numbering(document, first);
        Check((string?)firstShape.Attribute("HeadingType") == "Number" && (int?)firstShape.Attribute("Level") == 2 &&
            (int?)firstShape.Element("PARAMARGIN")!.Attribute("Left") == 1230,
            "An ordered list lost its native kind, depth or authoritative indentation.");
        Check((int?)firstDefinition.Attribute("Start") == 7 &&
            (int?)firstDefinition.Elements("PARAHEAD").Single(head => (int?)head.Attribute("Level") == 3).Attribute("Start") == 7,
            "The ordered segment did not start at its manuscript number.");
        var second = Paragraph();
        lists.Apply(second, start with { Number = 8, StartsList = false }, 30);
        Check(Numbering(document, second) == firstDefinition && (int?)firstDefinition.Attribute("Start") == 7,
            "An adjacent ordered item restarted its native counter or received a different definition.");

        var nested = Paragraph();
        lists.Apply(nested, new(2, "ordered", 3, 4, 4, true), 30);
        Check(Numbering(document, nested) != firstDefinition && (int?)Numbering(document, nested).Attribute("Start") == 4,
            "A nested list reused the parent counter.");
        var returning = Paragraph();
        lists.Apply(returning, start with { Number = 9, StartsList = false }, 30);
        Check(Numbering(document, returning) != firstDefinition && (int?)Numbering(document, returning).Attribute("Start") == 9,
            "Returning from a nested list did not resume the parent's current number in a separate segment.");
        lists.BreakSequence();
        var restart = Paragraph();
        lists.Apply(restart, start, 30);
        Check(Numbering(document, restart) != firstDefinition && (int?)Numbering(document, restart).Attribute("Start") == 7,
            "Equal native formats were merged across an explicit list restart.");
        foreach (var level in original.Elements("PARAHEAD"))
        {
            var actual = firstDefinition.Elements("PARAHEAD").Single(head => (string?)head.Attribute("Level") == (string?)level.Attribute("Level"));
            Check(actual.Value == level.Value && (string?)actual.Attribute("NumFormat") == (string?)level.Attribute("NumFormat"),
                "A native per-level numbering format or affix changed.");
            if ((int?)level.Attribute("Level") != 3)
                Check((string?)actual.Attribute("Start") == (string?)level.Attribute("Start"), "An unrelated level start was overwritten.");
        }
        Check(XNode.DeepEquals(layout.Ordered.Definition, original), "Ordered-list generation mutated the template definition.");
        Check((int?)document.Descendants("NUMBERINGLIST").Single().Attribute("Count") == document.Descendants("NUMBERING").Count(),
            "Generated numbering definitions left an inconsistent native count.");
    }

    private static void ContinuationsRetainParentIdentityAndFormatting()
    {
        var document = Fixture();
        var marker = new PreviewListMarker(3, "bullet", 2, 1, 1, true);
        var child = new PreviewListMarker(4, "bullet", 3, 1, 1, true);
        var plan = Plan(Continuation(marker), Continuation(child));
        var lists = new DirectXmlLists(document, plan, Layout(document));
        var parentParagraph = Paragraph();
        lists.Apply(parentParagraph, marker, 30);
        var childParagraph = Paragraph();
        lists.Apply(childParagraph, child, 30);
        var continuation = Paragraph();
        lists.ApplyContinuation(continuation, marker);
        var actual = Shape(document, continuation);
        var margin = actual.Element("PARAMARGIN")!;
        Check((string?)actual.Attribute("HeadingType") == "None" && (int?)actual.Attribute("Level") == 0 && actual.Attribute("Heading") is null &&
            (int?)margin.Attribute("Left") == 4130 && (int?)margin.Attribute("Indent") == 0,
            "A continuation followed the nested item, retained a native marker or lost its calculated hanging alignment.");
        Check(lists.Continuations.Margin(marker) == 4130 && lists.Continuations.Margin(child) == 4730 && !lists.NeedsNativeMeasurement,
            "Fixed-width markers required native display measurement or lost their separate continuation margins.");
        Check((string?)actual.Attribute("Align") == "Justify" && (int?)margin.Attribute("Right") == 60 &&
            (int?)margin.Attribute("Prev") == 222 && (int?)margin.Attribute("Next") == 111 && (int?)margin.Attribute("LineSpacing") == 180,
            "Continuation alignment changed unrelated paragraph formatting.");
        Check((string?)continuation.Attribute("Style") == "0" && TaggedTemplateBinding.DirectText(continuation) == "unchanged",
            "Applying a continuation changed its style or content.");

        var clearing = new XElement(parentParagraph);
        DirectXmlLists.Clear(clearing, document);
        var cleared = Shape(document, clearing);
        Check((string?)cleared.Attribute("HeadingType") == "None" && (int?)cleared.Attribute("Level") == 0 &&
            (int?)cleared.Element("PARAMARGIN")!.Attribute("Left") == 1230 && (int?)cleared.Element("PARAMARGIN")!.Attribute("Indent") == -100,
            "Clearing a marker changed the paragraph's native margins or indentation.");
        Reject(() => lists.ApplyContinuation(Paragraph(), marker with { Number = 99 }));

        var widthDocument = Fixture();
        var ordered = marker with { Kind = "ordered" };
        var measuring = new DirectXmlLists(widthDocument, Plan(Continuation(ordered)), Layout(widthDocument));
        measuring.Apply(Paragraph(), ordered, 30);
        measuring.ApplyContinuation(Paragraph(), ordered);
        Check(measuring.NeedsNativeMeasurement, "A native instance-width marker was approximated without measuring its actual display.");
    }

    private static void MarkerBackgroundsAndCanonicalAttributes()
    {
        var document = Fixture();
        var originals = document.Descendants("HEAD").Single().Descendants()
            .Where(element => element.Name is { LocalName: "PARASHAPE" or "CHARSHAPE" or "BORDERFILL" or "BULLET" or "NUMBERING" })
            .Select(element => (Name: element.Name, Id: (string)element.Attribute("Id")!, Copy: new XElement(element))).ToArray();
        var lists = new DirectXmlLists(document, Plan(), Layout(document));
        var paragraph = Paragraph();
        lists.Apply(paragraph, new(1, "bullet", 0, 1, 1, true), 30);
        var definition = document.Descendants("BULLET").Single(value => (string?)value.Attribute("Id") == (string?)Shape(document, paragraph).Attribute("Heading"));
        var character = document.Descendants("CHARSHAPE").Single(value => (string?)value.Attribute("Id") == (string?)definition.Element("PARAHEAD")!.Attribute("CharShape"));
        var border = document.Descendants("BORDERFILL").Single(value => (string?)value.Attribute("Id") == (string?)character.Attribute("BorderFillId"));
        Check(border.Element("FILLBRUSH") is null, "The default transparent marker brush did not match native list serialization.");
        var generated = document.Descendants("HEAD").Single().Descendants().Where(element => element.Attribute("Id") is not null &&
            !originals.Any(original => original.Name == element.Name && original.Id == (string?)element.Attribute("Id"))).ToArray();
        foreach (var element in generated.SelectMany(value => value.DescendantsAndSelf()))
            Check(element.Attributes().Select(attribute => attribute.Name.LocalName).SequenceEqual(
                element.Attributes().Select(attribute => attribute.Name.LocalName).Order(StringComparer.Ordinal)),
                "A generated native definition has noncanonical attribute ordering.");
        foreach (var original in originals)
            Check(XNode.DeepEquals(original.Copy, document.Descendants(original.Name).Single(value => (string?)value.Attribute("Id") == original.Id)),
                "Marker normalization modified an existing template definition.");
        var coloredDocument = Fixture();
        coloredDocument.Descendants("WINDOWBRUSH").Single().SetAttributeValue("FaceColor", "12345");
        var colored = new DirectXmlLists(coloredDocument, Plan(), Layout(coloredDocument));
        var coloredParagraph = Paragraph();
        colored.Apply(coloredParagraph, new(1, "bullet", 0, 1, 1, true), 30);
        var coloredDefinition = coloredDocument.Descendants("BULLET").Last();
        Check((string?)coloredDefinition.Element("PARAHEAD")!.Attribute("CharShape") == "0" &&
            (string?)coloredDocument.Descendants("WINDOWBRUSH").Single().Attribute("FaceColor") == "12345",
            "A real marker background was removed or replaced.");
        Reject(() => colored.Apply(Paragraph(), new(2, "ordered", 7, 1, 1, true), 30));
    }

    private static XDocument Fixture() => XDocument.Parse("""
        <HWPML><HEAD>
          <BORDERFILLLIST Count="2"><BORDERFILL Id="0"><LEFTBORDER Type="None"/></BORDERFILL>
            <BORDERFILL Id="1"><LEFTBORDER Type="None"/><FILLBRUSH><WINDOWBRUSH Alpha="0" FaceColor="4294967295" HatchColor="4278190080"/></FILLBRUSH></BORDERFILL></BORDERFILLLIST>
          <CHARSHAPELIST Count="1"><CHARSHAPE Id="0" Height="1000" BorderFillId="1"/></CHARSHAPELIST>
          <PARASHAPELIST Count="1"><PARASHAPE Id="0" Align="Justify" HeadingType="None" Level="0">
            <PARAMARGIN Left="30" Indent="-100" Right="60" Next="111" Prev="222" LineSpacing="180"/><PARABORDER BorderFill="0"/></PARASHAPE></PARASHAPELIST>
          <BULLETLIST Count="1"><BULLET Id="0" Char="•"><PARAHEAD CharShape="0" UseInstWidth="false" TextOffset="50" TextOffsetType="percent" WidthAdjust="0"/></BULLET></BULLETLIST>
          <NUMBERINGLIST Count="1"><NUMBERING Id="0" Start="2">
            <PARAHEAD CharShape="0" Level="1" NumFormat="Digit" Start="2" UseInstWidth="true">^1.</PARAHEAD>
            <PARAHEAD CharShape="0" Level="2" NumFormat="HangulSyllable" Start="3" UseInstWidth="true">^1-^2)</PARAHEAD>
            <PARAHEAD CharShape="0" Level="3" NumFormat="Digit" Start="4" UseInstWidth="true">(^3)</PARAHEAD>
            <PARAHEAD CharShape="0" Level="4" NumFormat="RomanCapital" Start="5" UseInstWidth="true">^4.</PARAHEAD>
            <PARAHEAD CharShape="0" Level="5" NumFormat="Digit" Start="6" UseInstWidth="true">^5.</PARAHEAD>
            <PARAHEAD CharShape="0" Level="6" NumFormat="Digit" Start="7" UseInstWidth="true">^6.</PARAHEAD>
            <PARAHEAD CharShape="0" Level="7" NumFormat="CircledDigit" Start="8" UseInstWidth="true">^7</PARAHEAD>
          </NUMBERING></NUMBERINGLIST>
        </HEAD><BODY><SECTION/></BODY></HWPML>
        """);

    private static ProfileListLayout Layout(XDocument document) => new(6, 600)
    {
        Bullet = new("bullet", 0, document.Descendants("BULLET").First()),
        Ordered = new("ordered", 0, document.Descendants("NUMBERING").First()),
    };
    private static IrPreviewPlan Plan(params PreviewOperation[] operations) => new("fixture", "fixture", new(operations.Length, operations.Length, 0, 0, 0), operations, []);
    private static PreviewOperation Continuation(PreviewListMarker marker) => new("text", "continuation", ["unchanged"], ParagraphStyle: "body", ListContinuation: marker);
    private static XElement Paragraph() => XElement.Parse("<P Style='0' ParaShape='0'><TEXT CharShape='0'><CHAR>unchanged</CHAR></TEXT></P>");
    private static XElement Shape(XDocument document, XElement paragraph) => document.Descendants("PARASHAPE").Single(value =>
        (string?)value.Attribute("Id") == (string?)paragraph.Attribute("ParaShape"));
    private static XElement Numbering(XDocument document, XElement paragraph) => document.Descendants("NUMBERING").Single(value =>
        (string?)value.Attribute("Id") == (string?)Shape(document, paragraph).Attribute("Heading"));
    private static void Check(bool value, string message) { if (!value) throw new Exception(message); }
    private static void Reject(Action operation)
    {
        try { operation(); } catch (InvalidOperationException) { return; }
        throw new Exception("Expected direct native list rejection.");
    }
}
