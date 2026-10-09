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
        XDocument beforeDocument, XDocument afterDocument)
    {
#if DEBUG
        return Equivalent(before.Select(p => TemplateFormatting.Copy(p, beforeDocument)).ToArray(),
            after.Select(p => TemplateFormatting.Copy(p, afterDocument)).ToArray());
#else
        var leftCopies = ExpandedCopies(before, beforeDocument);
        var rightCopies = ExpandedCopies(after, afterDocument);
        if (before.Count != after.Count) return false;
        var left = CanonicalizeOwned(leftCopies);
        var right = CanonicalizeOwned(rightCopies);
        return left.Zip(right).All(pair => XNode.DeepEquals(pair.First, pair.Second));
#endif
    }

    private static XElement[] ExpandedCopies(IReadOnlyList<XElement> roots, XDocument document)
    {
        var reader = new TemplateFormatting.ComparisonReader(document);
        return roots.Select(reader.Copy).ToArray();
    }

    private static XElement[] Canonicalize(IReadOnlyList<XElement> roots)
    {
        return CanonicalizeOwned(roots.Select(root => new XElement(root)).ToArray());
    }

    private static XElement[] CanonicalizeOwned(XElement[] copies)
    {
        // Hancom adds/removes empty character payloads in control-only and empty
        // paragraphs. Preserve TEXT formatting, all whitespace and actual text.
        copies.SelectMany(p => p.Descendants("CHAR"))
            .Where(c => c.Parent?.Name.LocalName == "TEXT" && !c.HasAttributes && !c.HasElements && c.Value.Length == 0)
            .Remove();
        // HWPML import combines adjacent payloads with identical formatting.
        // Keep every character, whitespace and ordered native control while
        // treating equivalent TEXT/CHAR segmentation as serialization detail.
        static bool SameAttributes(XElement left, XElement right) =>
            left.Attributes().OrderBy(a => a.Name.ToString(), StringComparer.Ordinal).Select(a => (a.Name, a.Value))
                .SequenceEqual(right.Attributes().OrderBy(a => a.Name.ToString(), StringComparer.Ordinal).Select(a => (a.Name, a.Value)));
        foreach (var paragraph in copies.SelectMany(root => root.DescendantsAndSelf("P")))
        {
            foreach (var run in paragraph.Elements("TEXT").ToArray())
                if (run.PreviousNode is XElement previous && previous.Name == run.Name && SameAttributes(previous, run))
                { previous.Add(run.Nodes().ToArray()); run.Remove(); }
            foreach (var character in paragraph.Elements("TEXT").Elements("CHAR").ToArray())
                if (character.PreviousNode is XElement previous && previous.Name == character.Name && SameAttributes(previous, character))
                { previous.Add(character.Nodes().ToArray()); character.Remove(); }
        }
        // HWPML specifies BothSides as the default TextFlow. Hancom may omit
        // the explicit default; all nondefault flow/wrap/position values stay exact.
        foreach (var shape in copies.SelectMany(root => root.Descendants("SHAPEOBJECT")))
            if ((string?)shape.Attribute("TextFlow") == "BothSides") shape.Attribute("TextFlow")!.Remove();
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
        // XML attribute order is not formatting. Newly assigned break flags may
        // be written in Hancom's canonical order on import/save.
        foreach (var element in copies.SelectMany(p => p.DescendantsAndSelf()))
        {
            var attributes = element.Attributes().OrderBy(a => a.Name.ToString(), StringComparer.Ordinal)
                .Select(a => new XAttribute(a)).ToArray();
            element.ReplaceAttributes(attributes);
        }
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

    public static string DescribeDifference(IReadOnlyList<XElement> before, IReadOnlyList<XElement> after,
        XDocument beforeDocument, XDocument afterDocument) => DescribeDifference(
#if DEBUG
            before.Select(p => TemplateFormatting.Copy(p, beforeDocument)).ToArray(),
            after.Select(p => TemplateFormatting.Copy(p, afterDocument)).ToArray());
#else
            ExpandedCopies(before, beforeDocument), ExpandedCopies(after, afterDocument));
#endif

    public static void RequireOriginalStyleDefinitions(XDocument before, XDocument after,
        IReadOnlyList<XElement>? preservedRoots = null)
    {
        // Named style identity remains stable; its formatting references need not.
        // Retained paragraphs are checked separately through Equivalent.
        var current = after.Descendants("STYLE").ToDictionary(e => (string)e.Attribute("Id")!);
#if !DEBUG
        var beforeReader = new TemplateFormatting.ComparisonReader(before);
        var afterReader = new TemplateFormatting.ComparisonReader(after);
#endif
        foreach (var original in before.Descendants("STYLE"))
        {
            var id = (string)original.Attribute("Id")!;
            if (!current.TryGetValue(id, out var saved) ||
#if DEBUG
                !XNode.DeepEquals(TemplateFormatting.Copy(original, before), TemplateFormatting.Copy(saved, after)))
#else
                !XNode.DeepEquals(beforeReader.Copy(original), afterReader.Copy(saved)))
#endif
                throw new InvalidOperationException($"Original style formatting changed: {(string?)original.Attribute("Name")} ({id}).");
        }
    }
}
