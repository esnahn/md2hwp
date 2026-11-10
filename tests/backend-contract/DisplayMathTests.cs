using System.Text.Json;
using System.Xml.Linq;
using Md2Hwp.HancomIrPreview;

internal static class DisplayMathTests
{
    internal static void Run()
    {
        void Check(bool condition, string message) { if (!condition) throw new Exception(message); }
        void Reject(Action action, string message)
        {
            try { action(); } catch (InvalidDataException) { return; }
            throw new Exception(message);
        }
        string Tag(string name) => TaggedTemplateBinding.Tag(name);
        XElement P(params object[] content) => new("P", new XAttribute("ParaShape", 0), new XAttribute("Style", 0),
            new XElement("TEXT", new XAttribute("CharShape", 0), content));
        XElement Text(string value) => P(new XElement("CHAR", value));
        XElement Number() => new("AUTONUM", new XAttribute("Number", 0), new XAttribute("NumberType", "Equation"),
            new XElement("AUTONUMFORMAT", new XAttribute("Superscript", "false"), new XAttribute("Type", "Digit")));
        XElement Sample() => P(new XElement("CHAR", TemplateDisplayMath.Slot, new XElement("TAB"), "("), Number(), new XElement("CHAR", ")"));
        XDocument Doc(XElement sample, string? size = "11.5", params XElement[] body)
        {
            var roots = new List<XElement> { Text(Tag("begin:template")) };
            if (size is not null) roots.Add(Text(Tag(TemplateDisplayMath.SizeSetting + ":" + size)));
            roots.AddRange([Text(Tag("begin:display_math")), sample, Text(Tag("end:display_math")), Text(Tag("end:template"))]);
            roots.AddRange(body);
            return new(new XElement("HWPML",
                new XElement("HEAD",
                    new XElement("DOCSETTING", new XElement("BEGINNUMBER", new XAttribute("Equation", 1))),
                    new XElement("MAPPINGTABLE",
                        new XElement("CHARSHAPELIST", new XElement("CHARSHAPE", new XAttribute("Id", 0), new XAttribute("Height", 1000))),
                        new XElement("PARASHAPELIST", new XElement("PARASHAPE", new XAttribute("Id", 0), new XAttribute("HeadingType", "None"))),
                        new XElement("STYLELIST", new XElement("STYLE", new XAttribute("Id", 0), new XAttribute("Type", "Para"),
                            new XAttribute("ParaShape", 0), new XAttribute("CharShape", 0), new XAttribute("Name", "body"))))),
                new XElement("BODY", new XElement("SECTION", roots))));
        }

        // Template declarations: exact size in 1/100 pt; definitions removed; source unchanged.
        var template = Doc(Sample());
        var original = template.ToString();
        var lowered = TemplateDisplayMath.Lower(template);
        Check(template.ToString() == original, "display_math lowering mutated the template.");
        Check(lowered.Layout.DisplayBaseUnit == 1150, "display_math.fontsize-pt was not converted to 1/100 pt.");
        Check(!lowered.Document.Descendants("P").Any(p => TaggedTemplateBinding.DirectText(p).Contains("display_math", StringComparison.Ordinal)),
            "display_math declarations survived lowering.");
        foreach (var size in new[] { "0", "0.00", "1.234", "-1", "1.", ".5", "abc", "1e3", "" })
            Reject(() => TemplateDisplayMath.Lower(Doc(Sample(), size)), "Invalid display_math size accepted: " + size);
        Reject(() => TemplateDisplayMath.Lower(Doc(Sample(), null)), "Missing display_math size accepted.");
        Reject(() => TemplateDisplayMath.Lower(Doc(P(new XElement("CHAR", "("), Number(), new XElement("CHAR", ") " + TemplateDisplayMath.Slot)))),
            "An Equation AUTONUM before the slot was accepted.");
        Reject(() => TemplateDisplayMath.Lower(Doc(Text(TemplateDisplayMath.Slot + TemplateDisplayMath.Slot))), "Two display slots accepted.");
        Reject(() => TemplateDisplayMath.Lower(Doc(P(new XElement("CHAR", TemplateDisplayMath.Slot),
            new XElement("AUTONUM", new XAttribute("NumberType", "Figure"))))), "A Figure AUTONUM was accepted in display math.");
        Reject(() => TemplateDisplayMath.Lower(Doc(P(new XElement("CHAR", "{{md2hwp:slot:", new XElement("TAB"), "display_math}}")))),
            "A slot crossing a tab was accepted.");
        Check(TemplateDisplayMath.Lower(Doc(Text(TemplateDisplayMath.Slot))).Layout.DisplayBaseUnit == 1150,
            "An unnumbered display sample was rejected.");

        // Attach: inline math uses its paragraph style size and no numbering;
        // display math clones the sample and renumbers the following AUTONUM.
        var inlineMarker = TemplateDisplayMath.InlineMarkerPrefix + new string('a', 32);
        var displays = new[] { TemplateDisplayMath.DisplayMarkerPrefix + new string('b', 32), TemplateDisplayMath.DisplayMarkerPrefix + new string('c', 32) };
        var rendered = TemplateDisplayMath.Lower(Doc(Sample(), "11.5",
            Text("앞 " + inlineMarker + " 뒤"), Text(displays[0]), Text(displays[1]))).Document;
        var runs = new[] { new PreviewTextRun("앞 ", false, false), new PreviewTextRun(inlineMarker, false, false, Math: "x^2"), new PreviewTextRun(" 뒤", false, false) };
        var operations = new List<PreviewOperation>
        {
            new("text", "body", ["앞 " + inlineMarker + " 뒤"], ParagraphStyle: "body", FormattedLines: [runs]),
        };
        foreach (var (marker, script) in displays.Zip(new[] { "y", "z" }))
            operations.Add(new("text", "display_math", [marker], ParagraphStyle: "body",
                FormattedLines: [[new PreviewTextRun(marker, false, false)]], DisplayMath: script));
        var plan = new IrPreviewPlan("ir.json", "test", new PreviewSummary(3, 3, 0, 0, 0), operations, []);
        IReadOnlyList<EquationKey>? requested = null;
        var attached = lowered.Layout.Attach(rendered, plan, keys =>
        {
            requested = keys;
            return keys.ToDictionary(k => k, k => new XElement("EQUATION", new XAttribute("BaseLine", 70), new XAttribute("BaseUnit", k.BaseUnit),
                new XElement("SHAPEOBJECT", new XAttribute("InstId", 7), new XAttribute("NumberingType", "Equation"), new XAttribute("ZOrder", 3),
                    new XElement("SIZE", new XAttribute("Width", 100), new XAttribute("Height", 50))),
                new XElement("SCRIPT", k.Script)));
        });
        Check(requested!.SequenceEqual([new EquationKey("y", 1150), new EquationKey("z", 1150), new EquationKey("x^2", 1000)]),
            "Equations were not measured once per distinct script and size.");
        var equations = attached.Descendants("EQUATION").ToArray();
        Check(equations.Length == 3, "Generated equation count changed.");
        var inline = equations.Single(e => (string?)e.Element("SCRIPT") == "x^2");
        Check((string?)inline.Element("SHAPEOBJECT")!.Attribute("NumberingType") == "None", "Inline math must not take an equation number.");
        Check(string.Concat(inline.Parent!.Elements().Select(e => e.Name.LocalName == "CHAR" ? e.Value : "|")) == "앞 | 뒤",
            "Inline math did not replace exactly its marker.");
        Check(equations.Where(e => (string?)e.Element("SCRIPT") != "x^2").All(e =>
            (string?)e.Element("SHAPEOBJECT")!.Attribute("NumberingType") == "Equation"), "Display math must keep Equation numbering.");
        Check(attached.Descendants("AUTONUM").Select(n => (int)n.Attribute("Number")!).SequenceEqual([1, 2]),
            "Display equation numbers were not recalculated.");
        Check(equations.Select(e => (string)e.Element("SHAPEOBJECT")!.Attribute("InstId")!).Distinct().Count() == 3 &&
            equations.All(e => (string)e.Element("SHAPEOBJECT")!.Attribute("InstId")! != "7"), "Equation identities were not freshly allocated.");
        Check(equations.Select(e => (int)e.Element("SHAPEOBJECT")!.Attribute("ZOrder")!).Distinct().Count() == 3,
            "Equation stacking orders were not freshly allocated.");
        Check(!attached.Descendants("P").Any(p => TaggedTemplateBinding.DirectText(p).Contains("MD2HWP_", StringComparison.Ordinal)),
            "A math marker survived attachment.");
        var display = equations.First(e => (string?)e.Element("SCRIPT") == "y").Parent!;
        Check(display.Elements().Select(e => e.Name.LocalName).SequenceEqual(["EQUATION", "CHAR", "AUTONUM", "CHAR"]) &&
            display.Elements("CHAR").First().Element("TAB") is not null, "Display sample affixes were not preserved after the equation.");

        // A leading tab (centered equation) stays before it without an empty text node.
        var centered = TemplateDisplayMath.Lower(Doc(P(new XElement("CHAR", new XElement("TAB"), TemplateDisplayMath.Slot, new XElement("TAB"), "("),
            Number(), new XElement("CHAR", ")")), "14", Text(displays[0])));
        var centeredPlan = new IrPreviewPlan("ir.json", "test", new PreviewSummary(1, 1, 0, 0, 0), [operations[1]], []);
        var centeredResult = centered.Layout.Attach(centered.Document, centeredPlan, keys => keys.ToDictionary(k => k, k =>
            new XElement("EQUATION", new XAttribute("BaseUnit", k.BaseUnit), new XElement("SHAPEOBJECT", new XAttribute("InstId", 7), new XAttribute("ZOrder", 1)), new XElement("SCRIPT", k.Script))));
        var centeredParagraph = centeredResult.Descendants("EQUATION").Single().Parent!;
        Check(centeredParagraph.Elements().Select(e => e.Name.LocalName).SequenceEqual(["CHAR", "EQUATION", "CHAR", "AUTONUM", "CHAR"]) &&
            centeredParagraph.Elements("CHAR").First().Nodes().Single() is XElement { Name.LocalName: "TAB" } &&
            (int)centeredResult.Descendants("EQUATION").Single().Attribute("BaseUnit")! == 1400,
            "A leading tab before the display slot was not preserved exactly.");

        // NEWNUM sets the next equation number; static Equation-numbered objects count.
        var numbered = XDocument.Parse("<HWPML><HEAD><DOCSETTING><BEGINNUMBER Equation=\"3\"/></DOCSETTING></HEAD><BODY><SECTION>" +
            "<P><TEXT><AUTONUM NumberType=\"Equation\" Number=\"9\"/><EQUATION><SHAPEOBJECT NumberingType=\"Equation\"/></EQUATION><AUTONUM NumberType=\"Equation\" Number=\"9\"/>" +
            "<EQUATION><SHAPEOBJECT NumberingType=\"None\"/></EQUATION><NEWNUM NumberType=\"Equation\" Number=\"10\"/>" +
            "<EQUATION><SHAPEOBJECT NumberingType=\"Equation\"/></EQUATION><AUTONUM NumberType=\"Equation\" Number=\"9\"/></TEXT></P></SECTION></BODY></HWPML>");
        TemplateDisplayMath.RecalculateNumbers(numbered);
        Check(numbered.Descendants("AUTONUM").Select(n => (int)n.Attribute("Number")!).SequenceEqual([2, 3, 10]),
            "Equation AUTONUM must show the preceding numbered equation's counter.");

        // Plan contexts: math only where paragraphs opt in, never in link labels.
        using var mathInline = JsonDocument.Parse("""[{"type":"inline_math","script":"x"}]""");
        Reject(() => InlineText.Read(mathInline.RootElement, "/h"), "Inline math accepted in a heading-like context.");
        Check(InlineText.Read(mathInline.RootElement, "/p", allowMath: true).Lines.Single().Runs.Single().Math == "x",
            "Inline math run lost its script.");
        using var linked = JsonDocument.Parse("""[{"type":"link","target":"https://example.net","title":null,"inlines":[{"type":"inline_math","script":"x"}]}]""");
        Reject(() => InlineText.Read(linked.RootElement, "/p", allowMath: true), "Inline math accepted in a link label.");
        Console.WriteLine("Display math declarations, native equation attachment, numbering and inline contexts passed.");
    }
}
