using System.Text.Json;
using System.Xml.Linq;
using Md2Hwp.HancomIrPreview;

internal static class ObjectSourceTests
{
    internal static void Run()
    {
        void Check(bool condition, string message) { if (!condition) throw new Exception(message); }
        void Reject(Action action) { try { action(); } catch (InvalidDataException) { return; } throw new Exception("Invalid object source was accepted."); }
        IReadOnlyList<PreviewSourceParagraph>? Read(string json) {
            using var input = JsonDocument.Parse(json);
            return PreviewSourceParagraph.ReadOptional(input.RootElement, "/blocks/0");
        }
        foreach (var label in new[] { "출처", "주.3", "NOTE1", "Source.", "trans.", "translation", "abbr" })
            Check(Read($$"""{"source":[{"prefix":"{{label}}","inlines":[{"type":"text","value":"설명"}]}]}""")!.Single().Prefix == label, "Source label was rewritten.");
        foreach (var label in new[] { "출처 ", "note1.", "note..1", "source1", "결과", "※", "rem" })
            Reject(() => Read($$"""{"source":[{"prefix":"{{label}}","inlines":[{"type":"text","value":"설명"}]}]}"""));
        Reject(() => Read("""{"source":[{"prefix":"주","inlines":[{"type":"footnote","blocks":[{"type":"paragraph","inlines":[{"type":"text","value":"note"}]}]}]}]}"""));
        Reject(() => Read("""{"source":[]}"""));
        Reject(() => Read("""{"source":[{"prefix":"주","inlines":[],"extra":1}]}"""));

        XElement P(string value) => new("P", new XAttribute("ParaShape", 0), new XAttribute("Style", 0),
            new XElement("TEXT", new XAttribute("CharShape", 0), new XElement("CHAR", value)));
        XDocument Doc(params XElement[] roots) => new(new XElement("HWPML",
            new XElement("HEAD", new XElement("PARASHAPELIST", new XElement("PARASHAPE", new XAttribute("Id",0), new XElement("PARAMARGIN", new XAttribute("Indent", -1200)))),
                new XElement("CHARSHAPELIST", new XElement("CHARSHAPE", new XAttribute("Id",0), new XAttribute("Height", 1000)))),
            new XElement("BODY", new XElement("SECTION", roots))));
        var sample = Doc(new[] { "code", "figure", "table" }.Select(role => P(TemplateObjectSources.PrefixSlot(role) + ": " + TaggedTemplateBinding.Tag("slot:" + role + ".source"))).ToArray());
        sample.Descendants("CHARSHAPELIST").Single().Add(new XElement("CHARSHAPE", new XAttribute("Id",1), new XAttribute("Height",800), new XElement("ITALIC")));
        var boxSample = sample.Descendants("P").First();
        var originalText = boxSample.Element("TEXT")!.Value;
        boxSample.ReplaceNodes(new XElement("TEXT", new XAttribute("CharShape",1), new XElement("CHAR", originalText[..10])),
            new XElement("TEXT",new XAttribute("CharShape",0),new XElement("CHAR",originalText[10..])));
        var before = sample.ToString();
        var lower = TemplateObjectSources.Lower(sample);
        Check(sample.ToString() == before, "Source lowering mutated the template.");
        Check(lower.Document.Descendants("P").All(p => TaggedTemplateBinding.DirectText(p).StartsWith("출처: ")), "Intermediate source labels were not lowered.");
        var missing = new XDocument(sample);
        missing.Descendants("CHAR").First().Value = "출처: " + TaggedTemplateBinding.Tag("slot:code.source");
        Reject(() => TemplateObjectSources.Lower(missing));
        var notes = Read("""{"source":[{"prefix":"주.3","inlines":[{"type":"strong","inlines":[{"type":"text","value":"설명"}]}]},{"prefix":"Source.","inlines":[{"type":"text","value":"기관"}]}]}""")!;
        XElement Caption(bool title) => new("CAPTION", new XElement("PARALIST", title ? new[] { P("[그림 1] 제목"), P("old source") } : new[] { P("old source") }));
        XElement Root(string shape, bool title) { var root = P(""); root.Element("TEXT")!.Add(new XElement(shape, new XElement("SHAPEOBJECT", Caption(title)))); return root; }
        var tableRoot = P(""); tableRoot.Element("TEXT")!.Add(new XElement("TABLE", new XElement("ROW",new XElement("CELL",new XElement("PARALIST",P("cell source"))))));
        var rendered = Doc(Root("PICTURE",true), Root("TABLE",false), tableRoot, P("unchanged"));
        rendered.Descendants("CHARSHAPELIST").Single().Add(new XElement(sample.Descendants("CHARSHAPE").Single(s => (string?)s.Attribute("Id") == "1")));
        var renderedBefore = rendered.ToString();
        var operations = new[] { "figure", "code", "table" }.Select(kind => new PreviewOperation(kind, kind, [], Sources: notes)).ToArray();
        var plan = new IrPreviewPlan("fixture", "fixture",new(3,0,1,1,0,1),operations,[]);
        var attached = lower.Layout.Attach(rendered,plan,0);
        Check(rendered.ToString() == renderedBefore, "Source attachment mutated its input.");
        var lists = attached.Descendants("PARALIST").ToArray();
        Check(lists[0].Elements("P").Select(TaggedTemplateBinding.DirectText).SequenceEqual(new[] { "[그림 1] 제목", "주.3: 설명", "Source.: 기관" }), "Figure caption lost its title or note order.");
        foreach (var list in lists.Skip(1))
            Check(list.Elements("P").Select(TaggedTemplateBinding.DirectText).SequenceEqual(new[] { "주.3: 설명", "Source.: 기관" }), "Box/table notes were not separate paragraphs.");
        Check(attached.Descendants("P").Last().Value == "unchanged", "Unrelated content changed.");
        var noteP = lists[0].Elements("P").ElementAt(1);
        var boldRun = noteP.Elements("TEXT").Single(t => t.Value == "설명");
        Check(attached.Descendants("CHARSHAPE").Single(s => (string?)s.Attribute("Id") == (string?)boldRun.Attribute("CharShape")).Element("BOLD") is not null, "Source emphasis was lost.");
        var boxPrefixRun = lists[1].Elements("P").First().Elements("TEXT").First();
        Check((string?)boxPrefixRun.Attribute("CharShape") == "1" && boxPrefixRun.Value == "주.3", "Split prefix tag lost its first-character formatting.");
        Check(lists.SelectMany(l => l.Elements("P")).All(p => (string?)p.Attribute("ParaShape") == "0"), "Source paragraph format changed.");
        Console.WriteLine("Object source prefixes, punctuation, closed IR, multiple native paragraphs and template formatting contracts passed.");
    }
}
