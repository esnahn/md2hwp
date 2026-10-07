using System.Xml.Linq;
using Md2Hwp.HancomIrPreview;

internal static class BoxPrototypeStructureTests
{
    internal static void Run()
    {
        var document = XDocument.Parse("""
            <HWPML><HEAD><CHARSHAPELIST><CHARSHAPE Id="1" Height="1000" TextColor="0"/><CHARSHAPE Id="2" Height="1200" TextColor="0"/></CHARSHAPELIST></HEAD>
            <BODY><SECTION><P><TEXT CharShape="1"><TABLE><SHAPEOBJECT TextFlow="BothSides" ZOrder="1" InstId="1"><SIZE Width="4000" Height="1000"/><POSITION HorzOffset="0"/></SHAPEOBJECT><ROW><CELL><PARALIST><P><TEXT CharShape="1"><CHAR>앞<LINEBREAK/>뒤</CHAR></TEXT></P></PARALIST></CELL></ROW></TABLE><CHAR/></TEXT></P></SECTION></BODY></HWPML>
            """);
        var original = document.Descendants("SECTION").Single().Element("P")!;
        var before = document.ToString();
        var normalized = new XDocument(document);
        var root = normalized.Descendants("SECTION").Single().Element("P")!;
        root.Element("TEXT")!.Element("CHAR")!.Remove();
        root.Descendants("SHAPEOBJECT").Single().Attribute("TextFlow")!.Remove();
        var character = root.Descendants("PARALIST").Single().Descendants("CHAR").Single();
        character.Parent!.ReplaceNodes(new XElement("CHAR", "앞"), new XElement("CHAR", new XElement("LINEBREAK")), new XElement("CHAR", "뒤"));
        foreach (var element in root.DescendantsAndSelf()) element.ReplaceAttributes(element.Attributes().Reverse().Select(a => new XAttribute(a)).ToArray());
        Check(AuriMinimalBoxPrototype.EquivalentOriginal(original, root), "Equivalent native box serialization was rejected.");
        Check(document.ToString() == before, "Box structure comparison modified its source.");
        void Reject(Action<XElement> change)
        {
            var changed = new XDocument(normalized);
            var changedRoot = changed.Descendants("SECTION").Single().Element("P")!;
            change(changedRoot);
            Check(!AuriMinimalBoxPrototype.EquivalentOriginal(original, changedRoot), "Real box prototype change escaped verification.");
        }
        Reject(p => p.Descendants("SIZE").Single().SetAttributeValue("Width", "5000"));
        var recalculated = new XDocument(normalized);
        var recalculatedRoot = recalculated.Descendants("SECTION").Single().Element("P")!;
        recalculatedRoot.Descendants("SIZE").Single().SetAttributeValue("Height", "2500");
        Check(AuriMinimalBoxPrototype.EquivalentOriginal(original, recalculatedRoot), "Native table height recalculation was mistaken for source loss.");
        Reject(p => p.Descendants("SIZE").Single().SetAttributeValue("Height", "0"));
        Reject(p => p.Descendants("SIZE").Single().SetAttributeValue("HeightRelTo", "Page"));
        Reject(p => p.Descendants("CELL").Single().SetAttributeValue("Height", "999"));
        Reject(p => p.Descendants("POSITION").Single().SetAttributeValue("HorzOffset", "1"));
        Reject(p => p.Descendants("LINEBREAK").Remove());
        Reject(p => p.Descendants("PARALIST").Single().Descendants("CHAR").Last().Value = "다름");
        Reject(p => p.Descendants("PARALIST").Single().Descendants("TEXT").Single().SetAttributeValue("CharShape", "2"));
        Reject(p => p.Descendants("SHAPEOBJECT").Single().SetAttributeValue("TextFlow", "LeftOnly"));
        Console.WriteLine("Box prototype serialization and calculated table height compare semantically; widths, cell heights, positions, content and formatting stay exact.");
    }
    private static void Check(bool value, string message) { if (!value) throw new Exception(message); }
}
