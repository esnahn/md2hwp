using System.Xml.Linq;
using Md2Hwp.HancomIrPreview;

internal static class TemplateConditionalRangesTests
{
    internal static void Run()
    {
        static string Tag(string name) => TaggedTemplateBinding.Tag(name);
        static XElement P(string text, string shape = "0") => new("P", new XAttribute("Style", "0"), new XAttribute("ParaShape", shape),
            new XElement("TEXT", new XAttribute("CharShape", "0"), new XElement("CHAR", text)));
        static string Text(IEnumerable<XElement> ps) => string.Concat(ps.Select(TaggedTemplateBinding.DirectText));
        var unrelatedEmpty = new XElement("P", new XAttribute("ParaShape", "0"));
        var unrelatedBefore = new XElement(unrelatedEmpty);
        TemplateOnceRanges.Apply([unrelatedEmpty], true);
        Check(XNode.DeepEquals(unrelatedEmpty, unrelatedBefore), "An unrelated native empty paragraph was changed.");
        foreach (var kind in new[] { "once", "except.once" })
        foreach (var first in new[] { true, false })
        {
            var include = first == (kind == "once");
            var inline = P("A" + Tag("begin:" + kind));
            inline.Add(new XElement("TEXT", new XAttribute("CharShape", "1"), new XElement("NEWNUM", new XAttribute("NumberType", "Page"), new XAttribute("Number", "7")),
                new XElement("CHAR", "B" + Tag("end:" + kind) + "C")));
            var inlines = TemplateOnceRanges.Apply([inline], first);
            Check(Text(inlines) == (include ? "ABC" : "AC") && inline.Descendants("NEWNUM").Any() == include, "Conditional text/control selection failed.");
            Check((string?)inline.Elements("TEXT").Last().Attribute("CharShape") == "1", "Inline formatting was lost.");

            var prefix = P("before" + Tag("begin:" + kind), "1");
            var next = P(Tag("end:" + kind) + "after", "2"); next.SetAttributeValue("PageBreak", "true");
            var output = TemplateOnceRanges.Apply([prefix, next], first);
            Check(output.Length == (include ? 2 : 1) && Text(output) == "beforeafter", "Conditional paragraph boundary did not join/remove tags.");
            Check(((string?)output.Last().Attribute("PageBreak") == "true") == include && (string?)output.Last().Attribute("ParaShape") == "2", "Break or destination formatting was lost.");

            var marker = P(Tag("begin:" + kind));
            next = P(Tag("end:" + kind) + "cover"); next.SetAttributeValue("PageBreak", "true");
            output = TemplateOnceRanges.Apply([marker, next], first);
            Check(output.Length == 1 && Text(output) == "cover" && ((string?)output[0].Attribute("PageBreak") == "true") == include, "Marker-only paragraph produced an extra blank paragraph.");
        }
        foreach (var kind in new[] { "once", "except.once" })
        foreach (var first in new[] { true, false })
        {
            var include = first == (kind == "once");
            var left = P("outside-left" + Tag("begin:" + kind) + "inside-left", "1");
            left.SetAttributeValue("PageBreak", "false");
            var middle = P("inside-middle"); middle.Element("TEXT")!.Add(new XElement("NEWNUM", new XAttribute("NumberType", "Page"), new XAttribute("Number", "9")));
            var right = P("inside-right" + Tag("end:" + kind) + "outside-right", "2");
            right.SetAttributeValue("PageBreak", "true"); right.SetAttributeValue("ColumnBreak", "true");
            var selected = TemplateOnceRanges.Apply([left, middle, right], first);
            Check(selected.Length == (include ? 3 : 1) && Text(selected) == (include ? "outside-leftinside-leftinside-middleinside-rightoutside-right" : "outside-leftoutside-right"), "Text across paragraphs was not conditional.");
            Check(((string?)selected.Last().Attribute("PageBreak") == "true") == include && ((string?)selected.Last().Attribute("ColumnBreak") == "true") == include, "Ending paragraph breaks were not included in the cross-paragraph range.");
            Check(selected.SelectMany(p => p.Descendants("NEWNUM")).Any() == include, "Cross-paragraph native controls were not conditional.");
            Check((string?)selected.Last().Attribute("ParaShape") == "2", "Ending paragraph formatting was lost.");
            var same = P("before" + Tag("begin:" + kind) + "conditional" + Tag("end:" + kind) + "after"); same.SetAttributeValue("PageBreak", "true");
            Check((string?)TemplateOnceRanges.Apply([same], first)[0].Attribute("PageBreak") == "true", "Same-paragraph ranges changed paragraph break properties.");
        }
        var cellLeft = new XElement("CELL", P(Tag("begin:except.once") + "text"));
        var cellRight = new XElement("CELL", P("text" + Tag("end:except.once")));
        var cells = P(""); cells.Element("TEXT")!.Add(new XElement("TABLE", new XElement("ROW", cellLeft, cellRight)));
        Reject(() => TemplateOnceRanges.Apply([cells], true));
        var split = P("{{md2hwp:begin:except.");
        split.Add(new XElement("TEXT", new XAttribute("CharShape", "1"), new XElement("CHAR", "once}}")));
        var splitEnd = P("{{md2hwp:end:except.");
        splitEnd.Add(new XElement("TEXT", new XAttribute("CharShape", "1"), new XElement("CHAR", "once}}cover")));
        Check(Text(TemplateOnceRanges.Apply([split, splitEnd], true)) == "cover", "Formatted boundary tags were not recognized.");
        var multiple = new[] { P(Tag("begin:once") + "first-only" + Tag("end:once") + Tag("begin:except.once")),
            P(Tag("end:except.once") + "middle" + Tag("begin:except.once")), P(Tag("end:except.once") + "last") };
        var joined = TemplateOnceRanges.Apply(multiple, true);
        Check(joined.Length == 1 && Text(joined) == "first-onlymiddlelast", "Separate inline ranges or adjacent conditional boundaries failed.");
        var nativePrefix = P(Tag("begin:except.once"));
        nativePrefix.Element("TEXT")!.AddFirst(new XElement("NEWNUM", new XAttribute("NumberType", "Figure"), new XAttribute("Number", "1")));
        var nativeMerged = TemplateOnceRanges.Apply([nativePrefix, P(Tag("end:except.once") + "cover")], true);
        Check(nativeMerged.Length == 1 && nativeMerged[0].Descendants("NEWNUM").Single().Attribute("Number")!.Value == "1", "Native controls were lost while joining paragraphs.");
        var container = new XElement("CELL", P(Tag("begin:except.once")), P(Tag("end:except.once") + "nested"));
        var anchor = P(""); anchor.Element("TEXT")!.Add(new XElement("TABLE", new XElement("ROW", container)));
        TemplateOnceRanges.Apply([anchor], true);
        Check(container.Elements("P").Count() == 1 && container.Value == "nested", "Nested paragraph boundary failed.");
        Reject(() => TemplateOnceRanges.Apply([P(Tag("begin:except.once") + "A" + Tag("end:once"))], true));
        Reject(() => TemplateOnceRanges.Apply([P(Tag("begin:once") + Tag("begin:except.once") + Tag("end:except.once") + Tag("end:once"))], true));

        Reject(() => TemplateOnceRanges.Apply([P(Tag("begin:except.once") + Tag("slot:heading1") + Tag("end:except.once"))], true));

        var crossed = P("{{md2hwp:begin:except."); crossed.Element("TEXT")!.Add(new XElement("TAB"), new XElement("CHAR", "once}}A" + Tag("end:except.once")));
        Reject(() => TemplateOnceRanges.Apply([crossed], true));

        XDocument Doc(IEnumerable<XElement> ps) => new(new XElement("HWPML", new XElement("HEAD",
            new XElement("PARASHAPELIST", new XElement("PARASHAPE", new XAttribute("Id", "0"))),
            new XElement("CHARSHAPELIST", new XElement("CHARSHAPE", new XAttribute("Id", "0")))), new XElement("BODY", new XElement("SECTION", ps))));
        var title = P(Tag("end:except.once") + Tag("slot:heading1")); title.SetAttributeValue("PageBreak", "true");
        var source = Doc([P(Tag("begin:template")), P(Tag("begin:heading1")), P(Tag("begin:except.once")), title,
            P(Tag("end:heading1")), P(Tag("end:template"))]);
        var before = source.ToString();
        var layout = TemplateHeadingBlocks.Lower(source).Layout;
        PreviewOperation Heading(string text) => new("text", text, [text], ParagraphStyle: "heading1", Heading1Number: 1);
        var plan = new IrPreviewPlan("fixture", "fixture", new(2, 2, 0, 0, 0), [Heading("first"), Heading("second")], []);
        var generated = layout.Attach(Doc([P("first"), P("second")]), plan, 0);
        var chapters = AuriMinimalBoxPrototype.RootParagraphs(generated).ToArray();
        Check(chapters.Length == 2 && (string?)chapters[0].Attribute("PageBreak") != "true" && (string?)chapters[1].Attribute("PageBreak") == "true", "Heading role occurrence did not select the correct break.");
        Check(source.ToString() == before, "Source template was mutated.");
        Console.WriteLine("Conditional once/except.once content, native controls, paragraph boundaries and heading occurrence tests passed.");
    }
    private static void Check(bool value, string message) { if (!value) throw new Exception(message); }
    private static void Reject(Action action) { try { action(); } catch (InvalidDataException) { return; } throw new Exception("Expected rejection."); }
}
