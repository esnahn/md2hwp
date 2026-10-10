using System.Xml.Linq;

namespace Md2Hwp.HancomIrPreview;

// Conditional content is parsed within one native paragraph container.
internal static class TemplateOnceRanges
{
    private sealed record Atom(XElement Node, char? Character, int Paragraph, bool Boundary = false);
    private static readonly string[] Tokens = ["begin:once", "end:once", "begin:except.once", "end:except.once"];

    internal static XElement[] Apply(IEnumerable<XElement> roots, bool first)
    {
        var paragraphs = roots.ToArray();
        var atoms = paragraphs.SelectMany((p, index) =>
            (index == 0 ? Enumerable.Empty<Atom>() : [new Atom(p, null, index, Boundary: true)])
            .Concat(p.Elements("TEXT").Elements().SelectMany(e => e.Name == "CHAR" && !e.HasElements
                ? e.Value.Select(c => new Atom(e, c, index)) : [new Atom(e, null, index)]))).ToArray();
        var keep = Enumerable.Repeat(true, atoms.Length).ToArray();
        var markerParagraphs = new HashSet<int>();
        string? inside = null;
        var beganAt = -1;
        var found = false;
        bool Matches(int start, string token) => start + token.Length <= atoms.Length &&
            token.Select((c, j) => atoms[start + j].Character == c).All(equal => equal);
        for (var i = 0; i < atoms.Length;)
        {
            var token = Tokens.FirstOrDefault(name => Matches(i, TaggedTemplateBinding.Tag(name)));
            if (token is not null)
            {
                var begin = token.StartsWith("begin:", StringComparison.Ordinal);
                var kind = token[(token.IndexOf(':') + 1)..];
                if (begin ? inside is not null : inside != kind)
                    throw new InvalidDataException("once/except.once ranges must pair within one container without nesting or mismatched boundaries.");
                if (begin) beganAt = atoms[i].Paragraph;
                else if (beganAt != atoms[i].Paragraph && LogicalText(paragraphs[beganAt]) == TaggedTemplateBinding.Tag("begin:" + kind))
                    markerParagraphs.Add(beganAt);
                Array.Fill(keep, false, i, TaggedTemplateBinding.Tag(token).Length);
                inside = begin ? kind : null;
                found = true;
                i += TaggedTemplateBinding.Tag(token).Length;
                continue;
            }
            if (inside is not null)
            {
                if (Matches(i, TaggedTemplateBinding.Prefix) || (!atoms[i].Boundary && atoms[i].Character is null &&
                    atoms[i].Node.Descendants("CHAR").Any(c => c.Value.Contains(TaggedTemplateBinding.Prefix, StringComparison.Ordinal))))
                    throw new InvalidDataException("once/except.once ranges cannot contain template declarations, title slots or nested ranges.");
                if (first != (inside == "once")) keep[i] = false;
            }
            i++;
        }
        if (inside is not null) throw new InvalidDataException("Conditional ranges must end within the same paragraph container.");

        var changed = atoms.Where((_, index) => !keep[index]).Select(a => paragraphs[a.Paragraph]).ToHashSet();
        var breaks = Enumerable.Repeat(true, paragraphs.Length).ToArray();
        foreach (var entry in atoms.Select((atom, index) => (atom, index)).Where(a => a.atom.Boundary))
            breaks[entry.atom.Paragraph] = keep[entry.index];
        if (found)
        {
            foreach (var group in atoms.Select((atom, index) => (atom, index)).Where(a => !a.atom.Boundary).GroupBy(a => a.atom.Node))
            {
                if (group.First().atom.Character is null)
                {
                    if (!keep[group.First().index]) group.Key.Remove();
                }
                else
                {
                    var text = new string(group.Where(a => keep[a.index]).Select(a => a.atom.Character!.Value).ToArray());
                    if (text.Length == 0) group.Key.Remove(); else group.Key.Value = text;
                }
            }
        }
        var result = new List<XElement>();
        for (var start = 0; start < paragraphs.Length;)
        {
            var end = start;
            while (end + 1 < paragraphs.Length && !breaks[end + 1]) end++;
            var destination = paragraphs[end];
            if (end > start)
            {
                if (paragraphs.Skip(start).Take(end - start + 1).Any(p => p.Elements().Any(e => e.Name != "TEXT")))
                    throw new InvalidDataException("Conditional paragraph joins require TEXT paragraphs in one container.");
                // The ending paragraph owns formatting; its preceding page/column
                // break is inside the omitted range, even when end is mid-paragraph.
                foreach (var flag in new[] { "PageBreak", "ColumnBreak" })
                    destination.SetAttributeValue(flag, (string?)paragraphs[start].Attribute(flag));
                var runs = paragraphs.Skip(start).Take(end - start).SelectMany(p => p.Elements("TEXT")).ToArray();
                foreach (var run in runs) run.Remove();
                destination.AddFirst(runs);
            }
            if (start != end || !markerParagraphs.Contains(start) || HasContent(destination) ||
                destination.Attributes().Any(a => a.Name.LocalName is "PageBreak" or "ColumnBreak" && a.Value == "true"))
                result.Add(destination);
            start = end + 1;
        }
        foreach (var paragraph in result)
        {
            Visit(paragraph);
            if (!changed.Contains(paragraph)) continue;
            var shape = (string?)paragraph.Element("TEXT")?.Attribute("CharShape") ?? "0";
            paragraph.Elements("TEXT").Where(t => !t.HasElements).Remove();
            if (!paragraph.Elements("TEXT").Any()) paragraph.Add(new XElement("TEXT", new XAttribute("CharShape", shape), new XElement("CHAR", "")));
        }
        return result.ToArray();

        void Visit(XElement element)
        {
            var originals = element.Elements("P").ToArray();
            if (originals.Length > 0)
            {
                var rewritten = Apply(originals, first);
                if (!originals.SequenceEqual(rewritten))
                {
                    if (element.Elements().Any(e => e.Name != "P"))
                        throw new InvalidDataException("Conditional paragraph ranges require a plain native paragraph container.");
                    foreach (var p in originals) p.Remove();
                    element.Add(rewritten);
                }
                return;
            }
            foreach (var child in element.Elements().ToArray()) Visit(child);
        }
    }

    private static string LogicalText(XElement paragraph) => string.Concat(paragraph.Elements("TEXT").Elements()
        .Select(e => e.Name == "CHAR" && !e.HasElements ? e.Value : "\0"));

    private static bool HasContent(XElement paragraph) => paragraph.Elements("TEXT").Elements()
        .Any(e => e.Name != "CHAR" || e.HasElements || e.Value.Length > 0);
}
