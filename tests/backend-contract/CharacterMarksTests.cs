using System.Text.Json;
using System.Xml.Linq;
using Md2Hwp.HancomIrPreview;

// Strikeout, superscript and subscript (IR 0.6) in the Hancom worker.
internal static class CharacterMarksTests
{
    internal static void Run()
    {
        void Check(bool condition, string message) { if (!condition) throw new Exception(message); }
        void Reject(string json, string expected)
        {
            try { _ = InlineText.Read(JsonDocument.Parse(json).RootElement, "/i", allowMath: true); }
            catch (InvalidDataException error) when (error.Message.Contains(expected, StringComparison.Ordinal)) { return; }
            throw new Exception("Expected rejection: " + expected);
        }
        const string Two = "{\"type\":\"text\",\"value\":\"2\"}";

        var content = InlineText.Read(JsonDocument.Parse(
            "[{\"type\":\"text\",\"value\":\"m\"},{\"type\":\"superscript\",\"inlines\":[" + Two + "]}," +
            "{\"type\":\"space\"},{\"type\":\"strikeout\",\"inlines\":[{\"type\":\"strong\",\"inlines\":[" +
            "{\"type\":\"text\",\"value\":\"H\"},{\"type\":\"subscript\",\"inlines\":[" + Two + "]}]}]}]").RootElement, "/i");
        var runs = content.Lines.Single().Runs;
        Check(runs.Select(run => (run.Text, run.Marks)).SequenceEqual(new[]
        {
            ("m", new RunMarks()),
            ("2", new RunMarks(Script: ScriptPosition.Superscript)),
            (" ", new RunMarks()),
            ("H", new RunMarks(Strong: true, Strikeout: true)),
            ("2", new RunMarks(Strong: true, Strikeout: true, Script: ScriptPosition.Subscript)),
        }), "Strikeout/superscript/subscript runs were not read from IR.");

        Reject("[{\"type\":\"superscript\",\"inlines\":[{\"type\":\"subscript\",\"inlines\":[" + Two + "]}]}]", "cannot be nested");
        Reject("[{\"type\":\"subscript\",\"inlines\":[{\"type\":\"inline_math\",\"script\":\"x\"}]}]", "inline math cannot be inside");
        Reject("[{\"type\":\"strikeout\",\"inlines\":[]}]", "must not be empty");
        Reject("[{\"type\":\"strikeout\",\"inlines\":[" + Two + "],\"color\":\"red\"}]", "color");

        // New formats follow Hancom 2020's own export: STRIKEOUT after UNDERLINE,
        // SUPERSCRIPT/SUBSCRIPT last, and a default solid black strikeout.
        XElement Shape(params XElement[] effects) => new("CHARSHAPE", new XAttribute("Height", 1000),
            new XElement("FONTID"), new XElement("RATIO"), new XElement("CHARSPACING"), new XElement("RELSIZE"),
            new XElement("CHAROFFSET"), effects);
        string Names(XElement shape) => string.Join(",", shape.Elements().Skip(5).Select(e => e.Name.LocalName));
        var underlined = Shape(new XElement("UNDERLINE", new XAttribute("Type", "Bottom")), new XElement("SHADOW"));
        DirectXmlParagraphs.ApplyMarks(underlined, new RunMarks(true, false, true, ScriptPosition.Superscript));
        Check(Names(underlined) == "BOLD,UNDERLINE,STRIKEOUT,SHADOW,SUPERSCRIPT", "Unexpected native mark order: " + Names(underlined));
        var strike = underlined.Element("STRIKEOUT")!;
        Check((string?)strike.Attribute("Type") == "Continuous" && (string?)strike.Attribute("Shape") == "Solid" &&
            (string?)strike.Attribute("Color") == "0", "Default strikeout differs from Hancom's command.");
        DirectXmlParagraphs.ApplyMarks(underlined, new RunMarks(Script: ScriptPosition.Subscript));
        Check(Names(underlined) == "BOLD,UNDERLINE,STRIKEOUT,SHADOW,SUBSCRIPT", "Subscript did not replace superscript.");

        var red = Shape(new XElement("STRIKEOUT", new XAttribute("Color", "255"), new XAttribute("Shape", "Dash"), new XAttribute("Type", "Continuous")));
        DirectXmlParagraphs.ApplyMarks(red, new RunMarks(Strikeout: true));
        Check((string?)red.Element("STRIKEOUT")!.Attribute("Color") == "255" && (string?)red.Element("STRIKEOUT")!.Attribute("Shape") == "Dash",
            "A template strikeout lost its own color or shape.");

        XElement WithId(XElement shape, int id) { shape.SetAttributeValue("Id", id); return shape; }
        var marks = HwpmlCharacterShapes.Read(new XDocument(new XElement("HWPML",
            WithId(Shape(new XElement("STRIKEOUT", new XAttribute("Type", "Continuous")), new XElement("SUBSCRIPT")), 1),
            WithId(Shape(new XElement("SUPERSCRIPT")), 2))));
        Check(marks[1] == new CharacterMarks(false, false, true, ScriptPosition.Subscript) &&
            marks[2] == new CharacterMarks(false, false, false, ScriptPosition.Superscript), "Saved character marks were not read back.");
        Console.WriteLine("Strikeout/superscript/subscript character-mark checks passed.");
    }
}
