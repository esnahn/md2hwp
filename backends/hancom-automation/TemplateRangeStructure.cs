using System.Globalization;
using System.Xml.Linq;

namespace Md2Hwp.HancomIrPreview;

internal static class TemplateRangeStructure
{
    // Hancom renumbers ZOrder on save. Preserve its complete relative order,
    // including ties; compare every other XML node and attribute exactly.
    public static bool Equivalent(IReadOnlyList<XElement> before, IReadOnlyList<XElement> after)
    {
        if (before.Count != after.Count) return false;
        var left = Canonicalize(before);
        var right = Canonicalize(after);
        return left.Zip(right).All(pair => XNode.DeepEquals(pair.First, pair.Second));
    }

    public static bool Equivalent(IReadOnlyList<XElement> before, IReadOnlyList<XElement> after,
        XDocument beforeDocument, XDocument afterDocument) =>
        Equivalent(before.Select(p => TemplateFormatting.Copy(p, beforeDocument)).ToArray(),
            after.Select(p => TemplateFormatting.Copy(p, afterDocument)).ToArray());

    private static XElement[] Canonicalize(IReadOnlyList<XElement> roots)
    {
        var copies = roots.Select(root => new XElement(root)).ToArray();
        // Hancom assigns fresh identities to pasted drawing objects and their
        // grouped components. Their geometry, content and stacking are checked.
        foreach (var attribute in copies.SelectMany(p => p.DescendantsAndSelf())
                     .Where(e => e.Name.LocalName is "P" or "SHAPEOBJECT" or "SHAPECOMPONENT")
                     .Attributes().Where(a => a.Name.LocalName is "InstId" or "InstID").ToArray())
            attribute.Remove();
        var orders = copies.SelectMany(root => root.DescendantsAndSelf())
            .Where(element => element.Name.LocalName == "SHAPEOBJECT")
            .Select(element => element.Attribute("ZOrder")).OfType<XAttribute>().ToArray();
        var values = orders.Select(order => int.Parse(order.Value, CultureInfo.InvariantCulture)).ToArray();
        var ranks = values.Distinct().Order().Select((value, rank) => (value, rank))
            .ToDictionary(pair => pair.value, pair => pair.rank);
        for (var i = 0; i < orders.Length; i++)
            orders[i].Value = ranks[values[i]].ToString(CultureInfo.InvariantCulture);
        return copies;
    }

    public static string DescribeDifference(IReadOnlyList<XElement> before, IReadOnlyList<XElement> after)
    {
        var left = Canonicalize(before).SelectMany(e => e.DescendantsAndSelf()).ToArray();
        var right = Canonicalize(after).SelectMany(e => e.DescendantsAndSelf()).ToArray();
        var differences = new List<string>();
        if (left.Length != right.Length) differences.Add($"elements {left.Length} -> {right.Length}");
        for (var i = 0; i < Math.Min(left.Length, right.Length) && differences.Count < 8; i++)
        {
            if (left[i].Name != right[i].Name) differences.Add($"element[{i}] {left[i].Name} -> {right[i].Name}");
            foreach (var name in left[i].Attributes().Select(a => a.Name).Union(right[i].Attributes().Select(a => a.Name)))
                if ((string?)left[i].Attribute(name) != (string?)right[i].Attribute(name))
                    differences.Add($"{left[i].Name}[{i}]/@{name}: {left[i].Attribute(name)?.Value} -> {right[i].Attribute(name)?.Value}");
            if (!left[i].HasElements && !right[i].HasElements && left[i].Value != right[i].Value)
                differences.Add($"{left[i].Name}[{i}] text differs");
        }
        return string.Join("; ", differences);
    }

    public static void RequireOriginalStyleDefinitions(XDocument before, XDocument after,
        IReadOnlyList<XElement>? preservedRoots = null)
    {
        // Named style identity remains stable; its formatting references need not.
        // Retained paragraphs are checked separately through Equivalent.
        var current = after.Descendants("STYLE").ToDictionary(e => (string)e.Attribute("Id")!);
        foreach (var original in before.Descendants("STYLE"))
        {
            var id = (string)original.Attribute("Id")!;
            if (!current.TryGetValue(id, out var saved) ||
                !XNode.DeepEquals(TemplateFormatting.Copy(original, before), TemplateFormatting.Copy(saved, after)))
                throw new InvalidOperationException($"Original style formatting changed: {(string?)original.Attribute("Name")} ({id}).");
        }
    }
}
