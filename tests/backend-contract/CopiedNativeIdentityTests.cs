// SPDX-License-Identifier: AGPL-3.0-only
using System.Xml.Linq;
using Md2Hwp.HancomIrPreview;

internal static class CopiedNativeIdentityTests
{
    internal static void Run()
    {
        var destination = new XDocument(new XElement("HWPML", new XElement("BODY", new XElement("SECTION",
            new XElement("P", new XAttribute("InstId", "4294967295")),
            new XElement("P", new XAttribute("InstID", "1"))))));
        var paragraph = new XElement("P", new XAttribute("InstId", "3971622632"),
            new XElement("TEXT", new XElement("FIELDBEGIN", new XAttribute("InstId", "2"),
                new XAttribute("FieldId", "627272811"), new XAttribute("Type", "Clickhere"),
                new XAttribute("Command", "parity-field")), new XElement("CHAR", "FIELD"),
                new XElement("FIELDEND", new XAttribute("FieldId", "627272811"), new XAttribute("Type", "Clickhere"))));
        var source = new XDocument(new XElement("HWPML", paragraph));
        var originalSource = source.ToString();
        var originalDestination = destination.ToString();
        // Keep every clone detached, exercising reservations that cannot be
        // recovered by scanning the destination XML on the next import.
        var copies = Enumerable.Range(0, 4).Select(_ => TemplateHeadingBlocks.ImportParagraph(paragraph, source, destination)).ToArray();
        Check(source.ToString() == originalSource && destination.ToString() == originalDestination,
            "Identity allocation changed source or static native identities.");
        var ids = copies.SelectMany(p => p.DescendantsAndSelf()).Attributes()
            .Where(a => a.Name.LocalName is "InstId" or "InstID").Select(a => uint.Parse(a.Value)).ToArray();
        Check(ids.All(id => id is > 2 and <= int.MaxValue) && ids.Distinct().Count() == ids.Length,
            "Generated identities collided with static/sample values or detached clones, or exceeded the adapter's positive 31-bit range.");
        foreach (var copy in copies)
        {
            var begin = copy.Descendants("FIELDBEGIN").Single();
            var end = copy.Descendants("FIELDEND").Single();
            Check((string?)begin.Attribute("FieldId") == "627272811" &&
                (string?)end.Attribute("FieldId") == "627272811" &&
                (string?)begin.Attribute("Command") == "parity-field" &&
                (string?)begin.Attribute("Type") == (string?)end.Attribute("Type"),
                "Identity allocation changed field pairing, type or command.");
        }
        var equivalentDestination = new XDocument(destination);
        var equivalent = TemplateHeadingBlocks.ImportParagraph(paragraph, source, equivalentDestination);
        Check(XNode.DeepEquals(copies[0], equivalent), "Equivalent document imports allocated different generated identities.");
        Console.WriteLine("Copied native identities are deterministic, unique across detached clones and limited to positive 31-bit values while static IDs and fields remain exact.");
    }

    private static void Check(bool condition, string message) { if (!condition) throw new Exception(message); }
}
