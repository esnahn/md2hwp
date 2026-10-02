using System.Globalization;
using System.Text;
using System.Text.Json;
using System.Text.RegularExpressions;
using System.Xml.Linq;

namespace Md2Hwp.HancomIrPreview;

internal static class TemplateMetadata
{
    private const string Prefix = "{{md2hwp:meta:";
    private static readonly Regex Tokens = new(@"\{\{md2hwp:meta:([^{}]+)\}\}", RegexOptions.CultureInvariant);
    private sealed record Atom(XElement Node, char Character);

    internal static Dictionary<string, string> Read(JsonElement metadata)
    {
        if (metadata.ValueKind != JsonValueKind.Object) throw new InvalidDataException("/metadata: expected object.");
        var values = new Dictionary<string,string>(StringComparer.Ordinal);
        foreach (var member in metadata.EnumerateObject())
        {
            var key = member.Name;
            if (key is not ("title" or "subtitle" or "author" or "date" or "date-meta" or "publisher") &&
                !(key.StartsWith("md2hwp-", StringComparison.Ordinal) && key.Length > 7))
                throw new InvalidDataException($"/metadata/{key}: unsupported metadata key.");
            if (key.Any(char.IsControl) || !key.IsNormalized(NormalizationForm.FormC))
                throw new InvalidDataException("Invalid metadata key.");
            string Text(JsonElement value)
            {
                if (value.ValueKind != JsonValueKind.String) throw new InvalidDataException($"/metadata/{key}: expected string.");
                var text = value.GetString()!;
                if (text.Any(char.IsControl) || !text.IsNormalized(NormalizationForm.FormC))
                    throw new InvalidDataException($"/metadata/{key}: expected one line of NFC text.");
                return text;
            }
            var value = member.Value;
            var text = key == "author" && value.ValueKind == JsonValueKind.Array
                ? value.GetArrayLength() > 0 ? string.Join(", ", value.EnumerateArray().Select(name => {
                        var author = Text(name);
                        if (string.IsNullOrWhiteSpace(author)) throw new InvalidDataException("/metadata/author: names must not be empty.");
                        return author;
                    }))
                    : throw new InvalidDataException("/metadata/author: list must not be empty.")
                : Text(value);
            if (!values.TryAdd(key, text)) throw new InvalidDataException($"Duplicate metadata key {key}.");
        }
        if (values.TryGetValue("date-meta", out var date))
        {
            if (!values.ContainsKey("date") || !DateOnly.TryParseExact(date, "yyyy-MM-dd", CultureInfo.InvariantCulture, DateTimeStyles.None, out _))
                throw new InvalidDataException("/metadata/date-meta: expected derived ISO date with original date.");
        }
        _ = TemplateHeadingNumbers.Start(values);
        return values;
    }

    internal static Dictionary<string,string> Load(string path)
    {
        var bytes = File.ReadAllBytes(path);
        if (bytes.Length > 8 * 1024 * 1024) throw new InvalidDataException("IR input exceeds 8 MiB.");
        using var document = JsonDocument.Parse(bytes);
        JsonContract.ValidateNoDuplicateMembers(document.RootElement, "");
        JsonContract.ExpectObject(document.RootElement, "", ["schema","ir_version","metadata","blocks"]);
        JsonContract.ExpectString(document.RootElement.GetProperty("schema"), "/schema", "md2hwp.ir");
        IrContract.RequireCurrent(JsonContract.RequiredString(document.RootElement,"ir_version",""), "Input");
        return Read(document.RootElement.GetProperty("metadata"));
    }

    internal sealed record Prepared(XDocument Document, IReadOnlyDictionary<string,string> Replacements)
    {
        internal XDocument Restore(XDocument source)
        {
            if (Replacements.Count == 0) return new XDocument(source);
            var tokens = new Regex(string.Join("|", Replacements.Keys.Select(Regex.Escape)), RegexOptions.CultureInvariant);
            return Transform(source, "MD2HWP_META_", tokens, match => Replacements[match.Value]);
        }
    }

    internal static Prepared Prepare(XDocument source, IReadOnlyDictionary<string,string> values)
    {
        var replacements = new Dictionary<string,string>(StringComparer.Ordinal);
        var result = Transform(source, Prefix, Tokens, match => {
            var text = Resolve(match.Groups[1].Value, values);
            var marker = "MD2HWP_META_" + Guid.NewGuid().ToString("N");
            replacements.Add(marker,text);
            return marker;
        });
        return new(result,replacements);
    }

    internal static XDocument Apply(XDocument source, IReadOnlyDictionary<string,string> values) =>
        Transform(source, Prefix, Tokens, match => Resolve(match.Groups[1].Value,values));

    private static string Resolve(string name, IReadOnlyDictionary<string,string> values)
    {
        var formatAt = name.IndexOf(":fmt:", StringComparison.Ordinal);
        var key = formatAt < 0 ? name : name[..formatAt];
        if (!values.TryGetValue(key, out var value) || string.IsNullOrWhiteSpace(value))
            throw new InvalidDataException($"Missing metadata {key}. Original date: {values.GetValueOrDefault("date", "(absent)")}.");
        if (formatAt < 0) return value;
        if (key != "date-meta") throw new InvalidDataException("fmt is supported only for date-meta.");
        return FormatDate(value,name[(formatAt+5)..]);
    }

    // Delayed replacements let native template binding inspect declarations
    // without interpreting literal tag-like text supplied in manuscript metadata.
    internal static XDocument Transform(XDocument source, string prefix, Regex tokens, Func<Match,string> resolve)
    {
        var document = new XDocument(source);
        foreach (var paragraph in document.Descendants("P").ToArray())
        {
            var atoms = paragraph.Elements("TEXT").SelectMany(t => t.Elements()).SelectMany(e =>
                e.Name.LocalName == "CHAR" && !e.HasElements
                    ? e.Value.Select(c => new Atom(e,c)) : new[] { new Atom(e,'\0') }).ToArray();
            var text = new string(atoms.Select(a => a.Character).ToArray());
            if (!text.Contains(prefix, StringComparison.Ordinal)) continue;
            var matches = tokens.Matches(text).Cast<Match>().ToArray();
            var keep = Enumerable.Repeat(true, atoms.Length).ToArray();
            var replacements = new Dictionary<int,string>();
            foreach (var match in matches)
            {
                if (match.Value.Contains('\0')) throw new InvalidDataException("Metadata tag cannot cross a native control.");
                string value;
                try { value = resolve(match); }
                catch (InvalidDataException error) {
                    throw new InvalidDataException($"{error.Message} Paragraph: '{TaggedTemplateBinding.DirectText(paragraph)}'.",error);
                }
                Array.Fill(keep, false, match.Index, match.Length);
                replacements.Add(match.Index,value);
            }
            var remaining = new string(text.Where((_,i) => keep[i]).ToArray());
            if (remaining.Contains(prefix,StringComparison.Ordinal))
                throw new InvalidDataException($"Malformed metadata tag in paragraph '{text}'.");
            foreach (var group in atoms.Select((atom,i) => (atom,i)).GroupBy(a => a.atom.Node))
            {
                if (group.Key.Name.LocalName != "CHAR") continue;
                var output = new StringBuilder();
                foreach (var (atom,i) in group)
                {
                    if (replacements.TryGetValue(i,out var replacement)) output.Append(replacement);
                    if (keep[i]) output.Append(atom.Character);
                }
                if (output.Length == 0)
                {
                    var parent = group.Key.Parent;
                    group.Key.Remove();
                    if (parent?.Name.LocalName == "TEXT" && !parent.HasElements) parent.Remove();
                }
                else group.Key.Value = output.ToString();
            }
        }
        return document;
    }

    internal static string FormatDate(string iso, string format)
    {
        if (!DateOnly.TryParseExact(iso,"yyyy-MM-dd",CultureInfo.InvariantCulture,DateTimeStyles.None,out var date))
            throw new InvalidDataException("date-meta must be an ISO date.");
        if (format.Length == 0 || format.Any(char.IsControl)) throw new InvalidDataException("fmt must be nonempty single-line text.");
        var output = new StringBuilder();
        for (var i=0;i<format.Length;i++)
        {
            if (format[i]!='%') { output.Append(format[i]); continue; }
            if (++i==format.Length) throw new InvalidDataException("Incomplete fmt specifier.");
            var unpadded = format[i]=='-';
            if (unpadded && ++i==format.Length) throw new InvalidDataException("Incomplete fmt specifier.");
            var token = format[i];
            string value = (token,unpadded) switch
            {
                ('Y',false) => date.Year.ToString("D4",CultureInfo.InvariantCulture),
                ('y',false) => (date.Year%100).ToString("D2",CultureInfo.InvariantCulture),
                ('m',false) => date.Month.ToString("D2",CultureInfo.InvariantCulture),
                ('d',false) => date.Day.ToString("D2",CultureInfo.InvariantCulture),
                ('m',true) => date.Month.ToString(CultureInfo.InvariantCulture),
                ('d',true) => date.Day.ToString(CultureInfo.InvariantCulture),
                ('F',false) => iso,
                ('%',false) => "%",
                _ => throw new InvalidDataException($"Unsupported fmt specifier %{(unpadded ? "-" : "")}{token}.")
            };
            output.Append(value);
        }
        return output.ToString();
    }
}
