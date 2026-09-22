using System.Xml.Linq;
using Md2Hwp.HancomIrPreview;

internal static class TaggedBindingChecks
{
    public static void Run(string identityFile)
    {
        static XElement P(string text, int style = 0) => new("P", new XAttribute("Style", style),
            new XElement("TEXT", new XAttribute("CharShape", 0), new XElement("CHAR", text)));
        static XElement T(string token, int style = 0) => P(TaggedTemplateBinding.Tag(token), style);
        var nativeStyles = Enumerable.Range(0, 12).Select(id => new XElement("STYLE",
            new XAttribute("Id", id), new XAttribute("Type", "Para"), new XAttribute("Name", "style-" + id),
            new XAttribute("CharShape", 0), new XAttribute("ParaShape", 0)));
        var section = new XElement("SECTION", P("static"), T("begin:samples"), T("contract:minimal-1"),
            T("body", 1), T("reset"),
            Enumerable.Range(1, 6).Select(level => T("heading." + level, level + 1)),
            T("figure.max-width-mm:142"), T("lists.max-depth:6"), T("lists.indent-hwp:2000"), T("source-label:출처:"),
            T("begin:block.box"),
            new XElement("P", new XAttribute("Style", 1), new XElement("TEXT", new XElement("TABLE",
                new XElement("CELL", T("slot:box.content", 9)), new XElement("CAPTION", P("출처: ", 11))))),
            T("end:block.box"), T("begin:figure"), T("slot:figure.image", 1),
            new XElement("P", new XAttribute("Style", 10), new XElement("TEXT", new XAttribute("CharShape", 0),
                new XElement("CHAR", "[그림 "), new XElement("AUTONUM", new XAttribute("NumberType", "Figure")),
                new XElement("CHAR", "] " + TaggedTemplateBinding.Tag("slot:figure.caption")))),
            T("slot:figure.source", 11), T("end:figure"), T("end:samples"), T("content"), P(""));
        var valid = new XDocument(new XElement("HWPML", new XElement("HEAD", nativeStyles,
            new XElement("CHARSHAPE", new XAttribute("Id", 0)),
            new XElement("PARASHAPE", new XAttribute("Id", 0), new XElement("PARAMARGIN", new XAttribute("Left", 0)))),
            new XElement("BODY", section)));
        static XElement Find(XDocument doc, string token) => doc.Descendants("P")
            .Single(p => TaggedTemplateBinding.DirectText(p) == TaggedTemplateBinding.Tag(token));
        var bound = TaggedTemplateBinding.Read(valid, identityFile);
        if (bound.Profile.ParagraphStyles.Single(s => s.Symbolic == "body").NativeName != "style-1" ||
            !bound.Profile.PreserveParagraphLineBreaks || bound.Profile.Figure.MaxWidthMillimeters != 142)
            throw new Exception("Native sample styles/settings were not bound.");
        var restyled = new XDocument(valid);
        // Changing a declared style definition does not require external profile changes.
        restyled.Descendants("STYLE").Single(e => (string?)e.Attribute("Id") == "1").SetAttributeValue("Name", "custom-body");
        if (TaggedTemplateBinding.Read(restyled, identityFile).Profile.ParagraphStyles.Single(s => s.Symbolic == "body").NativeName != "custom-body")
            throw new Exception("Binding ignored authored native style changes.");
        var count = 0;
        void Reject(string name, Action<XDocument> mutate)
        {
            var document = new XDocument(valid); mutate(document);
            try { TaggedTemplateBinding.Read(document, identityFile); }
            catch (InvalidDataException) { count++; return; }
            catch (InvalidOperationException) { count++; return; }
            throw new Exception($"Invalid tagged template accepted: {name}");
        }
        Reject("missing body", d => Find(d, "body").Remove());
        Reject("duplicate body", d => Find(d, "body").AddAfterSelf(new XElement(Find(d, "body"))));
        Reject("missing range end", d => Find(d, "end:block.box").Remove());
        Reject("content inside samples", d => { var p = Find(d, "content"); p.Remove(); Find(d, "body").AddAfterSelf(p); });
        Reject("unknown tag", d => Find(d, "body").AddAfterSelf(T("arbitrary")));
        Reject("nested extra range", d => Find(d, "slot:figure.image").AddAfterSelf(T("begin:block.box")));
        Reject("wrong box slot owner", d => { var p = Find(d, "slot:box.content"); p.Remove(); Find(d, "body").AddAfterSelf(p); });
        Reject("duplicate box slot", d => Find(d, "slot:box.content").AddAfterSelf(new XElement(Find(d, "slot:box.content"))));
        Reject("wrong image style", d => Find(d, "slot:figure.image").SetAttributeValue("Style", 10));
        Reject("caption control absent", d => d.Descendants("AUTONUM").Remove());
        Reject("extra caption control", d => d.Descendants("AUTONUM").Single().AddAfterSelf(new XElement("AUTONUM", new XAttribute("NumberType", "Table"))));
        Reject("line break in declaration", d => Find(d, "body").Element("TEXT")!.Add(new XElement("LINEBREAK")));
        Reject("line break in box slot", d => Find(d, "slot:box.content").Element("TEXT")!.Add(new XElement("LINEBREAK")));
        Reject("source not adjacent", d => Find(d, "slot:figure.source").AddBeforeSelf(P("gap")));
        Reject("width limit", d => Find(d, "figure.max-width-mm:142").ReplaceWith(T("figure.max-width-mm:143")));
        Reject("bad depth", d => Find(d, "lists.max-depth:6").ReplaceWith(T("lists.max-depth:7")));
        Reject("unknown version", d => Find(d, "contract:minimal-1").ReplaceWith(T("contract:minimal-2")));
        Reject("token in static region", d => d.Descendants("SECTION").Single().AddFirst(T("body")));
        Reject("nonempty terminal", d => d.Descendants("SECTION").Single().Elements().Last().ReplaceWith(P("suffix")));
        Reject("samples reversed", d => { var p = Find(d, "begin:samples"); p.Remove(); Find(d, "end:samples").AddAfterSelf(p); });
        Reject("two sections", d => d.Descendants("BODY").Single().Add(new XElement("SECTION", P(""))));
        Console.WriteLine($"Tagged native binding checks passed: 2 accepted, {count} rejected (synthetic HWPML; no COM).");
    }
}
