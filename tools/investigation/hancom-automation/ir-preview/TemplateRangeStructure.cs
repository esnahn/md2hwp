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
        Equivalent(ResolveBorderFills(before, beforeDocument), ResolveBorderFills(after, afterDocument));

    private static XElement[] ResolveBorderFills(IReadOnlyList<XElement> roots, XDocument document)
    {
        var definitions = document.Descendants().Where(e => e.Name.LocalName == "BORDERFILL")
            .ToDictionary(e => e.Attribute("Id")!.Value, e => e);
        var copies = roots.Select(root => new XElement(root)).ToArray();
        foreach (var attribute in copies.SelectMany(root => root.DescendantsAndSelf()).Attributes()
                     .Where(a => a.Name.LocalName is "BorderFill" or "BorferFill"))
        {
            if (!definitions.TryGetValue(attribute.Value, out var definition))
            {
                if (attribute.Value == "0") continue;
                throw new InvalidOperationException($"Unknown border-fill reference {attribute.Value}.");
            }
            var normalized = new XElement(definition);
            normalized.Attribute("Id")!.Remove();
            // Comparison-only dereference, never written back to an HWP file.
            attribute.Value = normalized.ToString(SaveOptions.DisableFormatting);
        }
        return copies;
    }

    private static XElement[] Canonicalize(IReadOnlyList<XElement> roots)
    {
        var copies = roots.Select(root => new XElement(root)).ToArray();
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
        var styles = before.Descendants().Where(e => e.Name.LocalName == "STYLE").ToArray();
        var preserved = (preservedRoots ?? []).SelectMany(e => e.DescendantsAndSelf()).ToArray();
        foreach (var name in new[] { "STYLE", "CHARSHAPE", "PARASHAPE" })
        {
            var current = after.Descendants().Where(e => e.Name.LocalName == name)
                .ToDictionary(e => e.Attribute("Id")!.Value, e => e);
            var reference = name == "CHARSHAPE" ? "CharShape" : "ParaShape";
            var required = styles.Concat(preserved).Select(e => (string?)e.Attribute(reference))
                .OfType<string>().ToHashSet(StringComparer.Ordinal);
            // Unused direct-format variants owned solely by removed samples may
            // be discarded/reused by Hancom. Preserve every named style and all
            // definitions referenced by it or by retained template content.
            foreach (var original in before.Descendants().Where(e => e.Name.LocalName == name &&
                         (name == "STYLE" || required.Contains(e.Attribute("Id")!.Value))))
                if (!current.TryGetValue(original.Attribute("Id")!.Value, out var saved) || !XNode.DeepEquals(original, saved))
                    throw new InvalidOperationException($"Original {name} definition changed: {original.Attribute("Id")?.Value}.");
        }
    }
}
