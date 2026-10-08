using System.Globalization;
using System.Text.RegularExpressions;
using System.Xml;
using System.Xml.Linq;
using System.Xml.XPath;

namespace Md2Hwp.HancomIrPreview;

// Insert documented ActionCrossRef fields only after the final HWPML import,
// when Hancom's native target identities are available. No UI actions are used.
internal sealed class NativeCrossReferences
{
    private static readonly Regex SourceMarker = new("MD2HWP_CROSS_REFERENCE_[0-9a-f]{32}", RegexOptions.CultureInvariant | RegexOptions.NonBacktracking);
    private readonly List<Reference> references = [];
    private readonly HashSet<string> sourceMarkers = new(StringComparer.Ordinal);
    private sealed record Reference(string Marker, string Kind, string Target, string TargetPath, string? Number, XElement Format,
        string ParagraphPath, int FieldOrdinal = -1);
    private sealed record Atom(XElement Element, int Offset, char Character);
    internal int Count => references.Count;

    internal static (NativeCrossReferences Layout, XDocument Document) Prepare(XDocument document, IrPreviewPlan plan,
        TemplateCrossReferences template, IReadOnlyDictionary<string, string> figureInstances,
        IReadOnlyDictionary<string, string>? tableInstances = null)
    {
        FigureReferenceContract.Validate(plan.Operations);
        var result = new XDocument(document);
        var layout = new NativeCrossReferences();
        var sources = new Dictionary<string, PreviewCrossReference>(StringComparer.Ordinal);
        foreach (var run in FigureReferenceContract.ReadRuns(plan.Operations))
        {
            if (run.CrossReference is not { } reference) continue;
            if (!SourceMarker.IsMatch(run.Text) || SourceMarker.Match(run.Text).Value != run.Text)
                throw new InvalidOperationException("Invalid IR reference placeholder.");
            if (sources.TryGetValue(run.Text, out var previous) && previous != reference)
                throw new InvalidOperationException("One reference placeholder names different targets.");
            sources[run.Text] = reference;
            layout.sourceMarkers.Add(run.Text);
        }
        // Static mixed CHAR/control payloads must remain byte-for-byte untouched.
        if (sources.Count == 0) return (layout, result);
        var objects = plan.Operations.Where(o => o.FigureId is not null || o.TableId is not null)
            .ToDictionary(o => o.FigureId ?? o.TableId!, StringComparer.Ordinal);
        var targets = new Dictionary<string, XElement>(StringComparer.Ordinal);
        foreach (var reference in sources.Values.Distinct())
        {
            if (reference.Kind is not ("figure_number" or "table_number"))
                throw new InvalidOperationException("Heading references must be resolved to fixed text before native object references.");
            var instances = reference.Kind == "figure_number" ? figureInstances : tableInstances;
            if (instances is null || !instances.TryGetValue(reference.Target, out var instance))
                throw new InvalidOperationException($"Missing generated native object identity for {reference.Target}.");
            var objectsWithIdentity = result.Descendants(reference.Kind == "figure_number" ? "PICTURE" : "TABLE")
                .Where(item => Identity(item.Element("SHAPEOBJECT")) == instance).ToArray();
            if (objectsWithIdentity.Length != 1) throw new InvalidOperationException($"Ambiguous native object target {reference.Target}.");
            var target = objectsWithIdentity[0];
            RequireObject(target, reference.Kind);
            targets[reference.Target] = target;
        }
        var found = new HashSet<string>(StringComparer.Ordinal);
        foreach (var paragraph in result.Descendants("P").ToArray())
        {
            if (!sources.Keys.Any(marker => TaggedTemplateBinding.DirectText(paragraph).Contains(marker, StringComparison.Ordinal))) continue;
            while (true)
            {
                SplitCharacters(paragraph);
                var atoms = Atoms(paragraph);
                var text = new string(atoms.Select(a => a.Character).ToArray());
                var matches = SourceMarker.Matches(text).Cast<Match>().Where(m => sources.ContainsKey(m.Value)).ToArray();
                if (matches.Length == 0) break;
                var match = matches[^1];
                if (!sources.TryGetValue(match.Value, out var reference)) throw new InvalidOperationException("Orphan IR cross-reference placeholder.");
                if (paragraph.Ancestors().Any(a => a.Name.LocalName is "HEADER" or "FOOTER" or "MASTERPAGE"))
                    throw new InvalidOperationException("Cross references cannot be copied into headers, footers or master pages.");
                var target = targets[reference.Target];
                var marker = "MD2HWP_NATIVE_NUMBER_REFERENCE_" + Guid.NewGuid().ToString("N");
                var context = atoms[match.Index].Element.Parent!;
                var fragments = reference.Kind switch
                {
                    "figure_number" => template.CreateFigureNumberFragments(result, objects[reference.Target].Heading1Number, marker, context),
                    "table_number" => template.CreateTableNumberFragments(result, objects[reference.Target].Heading1Number, marker, context),
                    _ => throw new InvalidOperationException("Unsupported native reference kind.")
                };
                Replace(paragraph, atoms.Skip(match.Index).Take(match.Length).ToArray(), fragments);
                Coalesce(paragraph);
                var location = FindMarker(paragraph, marker);
                layout.references.Add(new(marker, reference.Kind, reference.Target, Path(target),
                    ObjectNumber(target, reference.Kind),
                    Format(location.Atoms[0].Element.Parent!, result), Path(paragraph)));
                found.Add(match.Value);
            }
        }
        if (!found.SetEquals(sources.Keys) || result.Descendants("P").Any(p => sources.Keys.Any(marker => TaggedTemplateBinding.DirectText(p).Contains(marker, StringComparison.Ordinal))))
            throw new InvalidOperationException("A cross-reference placeholder was lost or crossed a native control boundary.");
        for (var i = 0; i < layout.references.Count; i++)
        {
            var reference = layout.references[i];
            var paragraphs = result.Descendants("P").Where(p => MarkerCount(p, reference.Marker) > 0).ToArray();
            if (paragraphs.Length != 1 || MarkerCount(paragraphs[0], reference.Marker) != 1)
                throw new InvalidOperationException($"Expected one prepared reference placeholder in {reference.ParagraphPath}; found {paragraphs.Length} paragraphs.");
            var atoms = Atoms(paragraphs[0]);
            var location = FindMarker(paragraphs[0], reference.Marker).Index;
            var text = new string(atoms.Take(location).Select(a => a.Character).ToArray());
            var ordinal = atoms.Take(location).Count(a => a.Element.Name.LocalName == "FIELDBEGIN") + layout.references.Count(r => text.Contains(r.Marker, StringComparison.Ordinal));
            layout.references[i] = reference with { ParagraphPath = Path(paragraphs[0]), FieldOrdinal = ordinal };
        }
        return (layout, result);
    }

    internal void Insert(dynamic hwp)
    {
        if (Count == 0) return;
        var imported = RenderProfile.ReadDocument((object)hwp);
        foreach (var reference in references)
        {
            var target = imported.XPathSelectElement(reference.TargetPath) ?? throw new InvalidOperationException("Native reference target moved during final import.");
            var instance = Instance(target, reference.Kind, imported);
            var paragraph = imported.XPathSelectElement(reference.ParagraphPath) ?? throw new InvalidOperationException("Native reference paragraph moved during final import.");
            var marker = FindMarker(paragraph, reference.Marker);
            if (!XNode.DeepEquals(reference.Format, Format(marker.Atoms[0].Element.Parent!, imported)))
                throw new InvalidOperationException("Native reference slot formatting changed during final import.");
            SelectMarker(hwp, reference.Marker, reference.ParagraphPath);
            dynamic characters = hwp.CreateAction("CharShape");
            dynamic characterSet = characters.CreateSet();
            characters.GetDefault(characterSet);
            if (!(bool)hwp.HAction.Run("Delete")) throw new InvalidOperationException("Could not remove native reference slot marker.");
            if (!(bool)characters.Execute(characterSet))
                throw new InvalidOperationException("Could not apply native reference character formatting.");
            dynamic action = hwp.CreateAction("InsertCrossReference");
            dynamic parameters = action.CreateSet();
            action.GetDefault(parameters);
            parameters.SetItem("Command", Command(instance, reference.Kind));
            if (!(bool)action.Execute(parameters))
                throw new InvalidOperationException("Could not insert the native number reference field.");
        }
        Verify(RenderProfile.ReadDocument((object)hwp));
    }

    private static void SelectMarker(dynamic hwp, string marker, string paragraphPath)
    {
        if (!(bool)hwp.HAction.Run("MoveDocBegin")) throw new InvalidOperationException("Could not begin native reference search.");
        dynamic set = hwp.HParameterSet.HFindReplace.HSet;
        hwp.HAction.GetDefault("RepeatFind", set);
        set.SetItem("FindString", marker); set.SetItem("Direction", hwp.FindDir("Forward"));
        set.SetItem("IgnoreMessage", 1); set.SetItem("MatchCase", 1); set.SetItem("SeveralWords", 0);
        set.SetItem("UseWildCards", 0); set.SetItem("WholeWordOnly", 0); set.SetItem("FindRegExp", 0);
        set.SetItem("IgnoreFindString", 0); set.SetItem("FindType", 1); set.SetItem("FindStyle", "");
        var found = (bool)hwp.HAction.Execute("RepeatFind", set);
        var selection = (int)hwp.SelectionMode;
        int list, paragraph, position;
        hwp.GetPos(out list, out paragraph, out position);
        var raw = (string)hwp.GetTextFile("UNICODE", "saveblock");
        try
        {
            if (!found) throw new InvalidOperationException("Native search did not find the slot.");
            // UNICODE export injects bullet/outline presentation text. HWPML's
            // selected direct CHAR payload proves the exact editable selection.
            RequireSelectedMarker(HwpMarkup.Parse((string)hwp.GetTextFile("HWPML2X", "saveblock")), marker);
        }
        catch (Exception error) when (error is InvalidOperationException or XmlException)
        {
            var codes = string.Join(" ", raw.Take(8).Select(c => $"U+{(int)c:X4}"));
            throw new InvalidOperationException($"Could not select the unique native reference slot: find={found} selection={selection} pos={list}/{paragraph}/{position} rawLength={raw.Length} normalizedLength={NormalizeSelectedText(raw).Length} expectedLength={marker.Length} prefixCodes={codes} paragraph={paragraphPath}. {error.Message}", error);
        }
    }

    internal static string NormalizeSelectedText(string text) => (text.StartsWith('\ufeff') ? text[1..] : text).TrimEnd('\r', '\n');

    internal static void RequireSelectedMarker(XDocument selected, string marker)
    {
        var root = selected.Root;
        var body = root?.Elements("BODY").ToArray() ?? [];
        var sections = body.Length == 1 ? body[0].Elements("SECTION").ToArray() : [];
        if (root?.Name.LocalName != "HWPML" || body.Length != 1 || sections.Length != 1 ||
            sections[0].Elements().Any(e => e.Name.LocalName != "P"))
            throw new InvalidOperationException("Selected markup has an unsupported document structure.");
        var paragraphs = sections[0].Elements("P").ToArray();
        if (paragraphs.Length != 1 || paragraphs[0].Elements().Any(e => e.Name.LocalName != "TEXT"))
            throw new InvalidOperationException("Selection must contain exactly one text paragraph.");
        var contents = paragraphs[0].Elements("TEXT").SelectMany(t => t.Elements()).ToArray();
        if (contents.Any(e => e.Name.LocalName is not ("CHAR" or "SECDEF" or "COLDEF") || e.Name.LocalName == "CHAR" && e.HasElements) ||
            string.Concat(contents.Where(e => e.Name.LocalName == "CHAR").Select(e => e.Value)) != marker)
            throw new InvalidOperationException("Selected native text does not exactly match the slot marker.");
    }

    internal void Verify(XDocument document)
    {
        if (Count == 0) return;
        var fieldIds = new HashSet<string>(StringComparer.Ordinal);
        foreach (var reference in references)
        {
            var target = document.XPathSelectElement(reference.TargetPath) ?? throw new InvalidOperationException("Saved native reference target is missing.");
            var instance = Instance(target, reference.Kind, document);
            var field = ReadField(document, reference);
            var command = (string?)field.Begin.Attribute("Command");
            if (!string.Equals((string?)field.Begin.Attribute("Type"), "Crossref", StringComparison.OrdinalIgnoreCase) ||
                !string.Equals((string?)field.End.Attribute("Type"), "Crossref", StringComparison.OrdinalIgnoreCase) ||
                command != Command(instance, reference.Kind) && command != Command(instance, reference.Kind) + ";")
                throw new InvalidOperationException("Native reference has the wrong target, reference kind, hyperlink or options.");
            if ((string?)field.Begin.Attribute("Editable") != "false" || (string?)field.End.Attribute("Editable") != "false" ||
                (string?)field.Begin.Attribute("Dirty") != "false" || (string?)field.Begin.Attribute("Property") != "0" || (string?)field.End.Attribute("Property") != "0")
                throw new InvalidOperationException("Native reference field flags changed.");
            var id = Identity(field.Begin);
            if (!uint.TryParse(id, NumberStyles.None, CultureInfo.InvariantCulture, out _) || !fieldIds.Add(id!) ||
                document.Descendants("FIELDBEGIN").Count(e => Identity(e) == id) != 1)
                throw new InvalidOperationException("Native reference field identity is missing or duplicated.");
            var number = reference.Number;
            if (number is null || field.Text != number || ObjectNumber(target, reference.Kind) != number)
                throw new InvalidOperationException("Native reference display does not match its target number.");
            foreach (var fragment in field.Fragments)
                if (!XNode.DeepEquals(reference.Format, Format(fragment, document)))
                    throw new InvalidOperationException("Native reference number lost its source inline character formatting.");
        }
        if (document.Descendants("P").Any(p => sourceMarkers.Any(m => TaggedTemplateBinding.DirectText(p).Contains(m, StringComparison.Ordinal)) || references.Any(r => TaggedTemplateBinding.DirectText(p).Contains(r.Marker, StringComparison.Ordinal))))
            throw new InvalidOperationException("An unresolved native cross-reference marker remains.");
    }

    internal void NormalizeExpected(XDocument expected, XDocument actual)
    {
        Verify(actual);
        foreach (var reference in references)
        {
            var paragraph = expected.XPathSelectElement(reference.ParagraphPath) ?? throw new InvalidOperationException("Expected reference paragraph is missing.");
            SplitCharacters(paragraph);
            var marker = FindMarker(paragraph, reference.Marker);
            var format = marker.Atoms[0].Element.Parent!;
            var fragments = ReadField(actual, reference).Fragments.Select(t => new XElement("TEXT", format.Attributes(), t.Elements().Select(e => new XElement(e)))).ToArray();
            Replace(paragraph, marker.Atoms, fragments);
            Coalesce(paragraph);
        }
    }

    internal static string Command(string instance, string kind) => $"?#{instance};{(kind switch { "table_number" => 0, "figure_number" => 1, _ => throw new InvalidOperationException("Unsupported native reference kind.") })};1;0;0";

    private static string Instance(XElement target, string kind, XDocument document)
    {
        RequireObject(target, kind);
        var owner = target.Element("SHAPEOBJECT")!;
        var instance = Identity(owner);
        if (!uint.TryParse(instance, NumberStyles.None, CultureInfo.InvariantCulture, out _) ||
            document.Descendants().Count(e => Identity(e) == instance) != 1)
            throw new InvalidOperationException("Native reference target identity is missing or ambiguous.");
        return instance!;
    }

    private static string? Identity(XElement? element) => (string?)element?.Attribute("InstId") ?? (string?)element?.Attribute("InstID");

    private static void RequireObject(XElement picture, string kind)
    {
        if (picture.Name.LocalName != (kind == "table_number" ? "TABLE" : "PICTURE") ||
            (string?)picture.Element("SHAPEOBJECT")?.Attribute("NumberingType") != (kind == "table_number" ? "Table" : "Figure"))
            throw new InvalidOperationException("Native reference target is not a numbered object with a caption.");
    }

    private static string ObjectNumber(XElement picture, string kind)
    {
        var counters = picture.Element("SHAPEOBJECT")?.Element("CAPTION")?.Descendants("AUTONUM").Where(e => (string?)e.Attribute("NumberType") == (kind == "table_number" ? "Table" : "Figure")).ToArray() ?? [];
        if (counters.Length != 1 || !int.TryParse((string?)counters[0].Attribute("Number"), NumberStyles.None, CultureInfo.InvariantCulture, out var number) || number < 1)
            throw new InvalidOperationException("Referenced object needs one positive native AUTONUM in its caption.");
        return number.ToString(CultureInfo.InvariantCulture);
    }

    private static (XElement Begin, XElement End, string Text, XElement[] Fragments) ReadField(XDocument document, Reference reference)
    {
        var paragraph = document.XPathSelectElement(reference.ParagraphPath) ?? throw new InvalidOperationException("Saved reference paragraph is missing.");
        var elements = paragraph.Elements("TEXT").SelectMany(t => t.Elements()).ToArray();
        var starts = elements.Where(e => e.Name.LocalName == "FIELDBEGIN").ToArray();
        if (reference.FieldOrdinal < 0 || reference.FieldOrdinal >= starts.Length) throw new InvalidOperationException("Saved native reference field is missing.");
        var begin = starts[reference.FieldOrdinal];
        var first = Array.IndexOf(elements, begin);
        var payload = new List<XElement> { begin };
        XElement? end = null;
        foreach (var element in elements.Skip(first + 1))
        {
            payload.Add(element);
            if (element.Name.LocalName == "FIELDEND") { end = element; break; }
            if (element.Name.LocalName != "CHAR" || element.HasElements) throw new InvalidOperationException("Native reference cache crosses a control boundary.");
        }
        if (end is null) throw new InvalidOperationException("Native reference FIELDEND is missing.");
        // Hancom reuses FieldId for separate native references to the same
        // target. Pair only the local begin/end; InstId owns control identity.
        var fieldId = (string?)begin.Attribute("FieldId");
        if (!uint.TryParse(fieldId, NumberStyles.None, CultureInfo.InvariantCulture, out _) ||
            fieldId != (string?)end.Attribute("FieldId") ||
            !string.Equals((string?)begin.Attribute("Type"), (string?)end.Attribute("Type"), StringComparison.OrdinalIgnoreCase))
            throw new InvalidOperationException("Native reference begin/end pairing is invalid.");
        var fragments = new List<XElement>();
        XElement? sourceText = null;
        foreach (var element in payload)
        {
            if (element.Parent != sourceText)
            {
                sourceText = element.Parent;
                fragments.Add(new XElement("TEXT", sourceText!.Attributes()));
            }
            fragments[^1].Add(new XElement(element));
        }
        return (begin, end, string.Concat(payload.Where(e => e.Name.LocalName == "CHAR").Select(e => e.Value)), fragments.ToArray());
    }

    private static XElement Format(XElement text, XDocument document) => TemplateFormatting.Copy(new XElement("TEXT", text.Attributes()), document);
    private static string Path(XElement element) => "/" + string.Join("/", element.AncestorsAndSelf().Reverse().Select(e => $"{e.Name.LocalName}[{e.ElementsBeforeSelf(e.Name).Count() + 1}]"));

    private static List<Atom> Atoms(XElement paragraph)
    {
        var result = new List<Atom>();
        foreach (var element in paragraph.Elements("TEXT").SelectMany(t => t.Elements()))
        {
            if (element.Name.LocalName != "CHAR") { result.Add(new(element, 0, '\0')); continue; }
            var offset = 0;
            foreach (var node in element.Nodes())
            {
                if (node is XText text) foreach (var character in text.Value) result.Add(new(element, offset++, character));
                else if (node is XElement control) { result.Add(new(control, 0, '\0')); offset += control.Value.Length; }
            }
        }
        return result;
    }

    private static int MarkerCount(XElement paragraph, string marker)
    {
        var text = new string(Atoms(paragraph).Select(a => a.Character).ToArray());
        var count = 0;
        for (var index = text.IndexOf(marker, StringComparison.Ordinal); index >= 0; index = text.IndexOf(marker, index + marker.Length, StringComparison.Ordinal)) count++;
        return count;
    }

    private static (int Index, Atom[] Atoms) FindMarker(XElement paragraph, string marker)
    {
        var atoms = Atoms(paragraph);
        var text = new string(atoms.Select(a => a.Character).ToArray());
        var index = text.IndexOf(marker, StringComparison.Ordinal);
        if (index < 0 || text.IndexOf(marker, index + marker.Length, StringComparison.Ordinal) >= 0)
            throw new InvalidOperationException($"Expected one native reference marker in {Path(paragraph)}.");
        return (index, atoms.Skip(index).Take(marker.Length).ToArray());
    }

    private static void SplitCharacters(XElement paragraph)
    {
        foreach (var character in paragraph.Elements("TEXT").Elements("CHAR").Where(c => c.HasElements).ToArray())
        {
            var pieces = new List<XElement>();
            foreach (var node in character.Nodes().ToArray())
            {
                node.Remove();
                pieces.Add(new XElement("CHAR", character.Attributes(), node));
            }
            character.ReplaceWith(pieces);
        }
    }

    private static void Replace(XElement paragraph, IReadOnlyList<Atom> span, IEnumerable<XElement> fragments)
    {
        if (span.Count == 0) throw new InvalidOperationException("Empty native reference span.");
        var first = span[0]; var last = span[^1];
        if (first.Element.Name.LocalName != "CHAR" || last.Element.Name.LocalName != "CHAR" || first.Element.HasElements || last.Element.HasElements)
            throw new InvalidOperationException("Native reference marker crosses a control boundary.");
        var firstText = first.Element.Parent!; var lastText = last.Element.Parent!;
        var before = new XElement("TEXT", firstText.Attributes());
        foreach (var node in first.Element.NodesBeforeSelf().ToArray()) { node.Remove(); before.Add(node); }
        if (first.Offset > 0) before.Add(new XElement("CHAR", first.Element.Value[..first.Offset]));
        var after = new XElement("TEXT", lastText.Attributes());
        if (last.Offset + 1 < last.Element.Value.Length) after.Add(new XElement("CHAR", last.Element.Value[(last.Offset + 1)..]));
        foreach (var node in last.Element.NodesAfterSelf().ToArray()) { node.Remove(); after.Add(node); }
        var texts = paragraph.Elements("TEXT").ToArray();
        var start = Array.IndexOf(texts, firstText); var finish = Array.IndexOf(texts, lastText);
        firstText.AddBeforeSelf(before, fragments.Select(f => new XElement(f)), after);
        for (var i = start; i <= finish; i++) texts[i].Remove();
    }

    private static void Coalesce(XElement paragraph)
    {
        foreach (var character in paragraph.Elements("TEXT").Elements("CHAR").Where(c => !c.HasElements && c.Value.Length == 0).ToArray()) character.Remove();
        foreach (var text in paragraph.Elements("TEXT").ToArray())
        {
            if (!text.HasElements) { text.Remove(); continue; }
            if (text.PreviousNode is XElement previous && previous.Name == text.Name && XNode.DeepEquals(new XElement("TEXT", previous.Attributes()), new XElement("TEXT", text.Attributes())))
            {
                foreach (var node in text.Nodes().ToArray()) { node.Remove(); previous.Add(node); }
                text.Remove();
            }
        }
        foreach (var text in paragraph.Elements("TEXT"))
        foreach (var character in text.Elements("CHAR").ToArray())
            if (character.PreviousNode is XElement previous && previous.Name == character.Name && !previous.HasAttributes && !character.HasAttributes)
            {
                foreach (var node in character.Nodes().ToArray()) { node.Remove(); previous.Add(node); }
                character.Remove();
            }
    }
}
