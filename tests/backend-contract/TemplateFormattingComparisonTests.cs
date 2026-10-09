using System.Xml.Linq;
using Md2Hwp.HancomIrPreview;

internal static class TemplateFormattingComparisonTests
{
    internal static void Run()
    {
        NestedReferencesAndDocumentResolution();
        MissingAndDuplicateDefinitions();
        ReaderIsolationAndFreshScopes();
        StructuralComparisonPreservesInputsAndRejectsChanges();
        Console.WriteLine("Scoped formatting comparisons retain recursive definitions, reference errors and strict structure without modifying input XML.");
    }

    private static void NestedReferencesAndDocumentResolution()
    {
        var document = Fixture();
        var original = document.ToString(SaveOptions.DisableFormatting);
        var source = Root(document);
        var expected = ExpectedParagraph();
        var reader = new TemplateFormatting.ComparisonReader(document);
        Equal(expected, reader.Copy(source), "Nested formatting definitions were not expanded by value.");
        Equal(expected, TemplateFormatting.Copy(source), "Attached elements lost their source.Document fallback.");
        Equal(expected, new TemplateFormatting.ComparisonReader(null).Copy(source), "A reader without an explicit document lost source.Document fallback.");
        Equal(expected, reader.Copy(new XElement(source)), "Explicit documents did not resolve detached elements.");
        Equal(expected, TemplateFormatting.Copy(new XElement(source), document), "Detached elements with an explicit document changed their formatting.");
        Equal(TemplateFormatting.Copy(source), reader.Copy(source), "Legacy and indexed formatting copies differ.");
        Equal(TemplateFormatting.Copy(document.Descendants("PARASHAPE").Single()),
            reader.Copy(document.Descendants("PARASHAPE").Single()), "Copying a definition itself changed its identity or nested formatting.");
        Check((string?)reader.Copy(document.Descendants("PARASHAPE").Single()).Attribute("Id") == "7",
            "Top-level definition copies removed their own Id.");

        var bullet = new XElement("PARASHAPE", new XAttribute("Id", "8"), new XAttribute("HeadingType", "Bullet"), new XAttribute("Heading", "3"));
        var copiedBullet = reader.Copy(bullet);
        Equal(new XElement("BULLET", new XAttribute("Char", "●"), new XAttribute("CharShape", Xml(ExpectedCharacter()))),
            XElement.Parse((string)copiedBullet.Attribute("Heading")!), "Bullet Heading references resolved against NUMBERING.");
        Equal(TemplateFormatting.Copy(bullet, document), copiedBullet, "Legacy and indexed bullet resolution differ.");

        var expanded = new XElement("P", new XAttribute("ParaShape", "<already-expanded>"),
            new XElement("TEXT", new XAttribute("CharShape", "<not-reparsed>"), new XElement("CHAR", "문자")));
        Equal(expanded, new TemplateFormatting.ComparisonReader(null).Copy(expanded), "Already-expanded reference values were interpreted again.");
        Equal(new XElement("P", new XElement("TEXT", new XElement("CHAR", "서식 참조 없음"))),
            new TemplateFormatting.ComparisonReader(null).Copy(new XElement("P", new XElement("TEXT", new XElement("CHAR", "서식 참조 없음")))),
            "A detached element without formatting references unnecessarily required a document.");
        Check(Xml(document) == original, "Formatting expansion modified its source document.");
    }

    private static void MissingAndDuplicateDefinitions()
    {
        var document = Fixture();
        var reader = new TemplateFormatting.ComparisonReader(document);
        Reject(() => reader.Copy(new XElement("TEXT", new XAttribute("CharShape", "404"))), "Unknown CHARSHAPE reference 404.");
        Reject(() => TemplateFormatting.Copy(new XElement("TEXT", new XAttribute("CharShape", "404")), document), "Unknown CHARSHAPE reference 404.");
        Reject(() => new TemplateFormatting.ComparisonReader(null).Copy(new XElement("P", new XAttribute("ParaShape", "7"))),
            "Missing document for ParaShape formatting comparison.");
        Reject(() => TemplateFormatting.Copy(new XElement("P", new XAttribute("ParaShape", "7"))),
            "Missing document for ParaShape formatting comparison.");
        Equal(new XElement("CELL", new XAttribute("BorderFill", "0"), new XAttribute("BorferFill", "0"), new XAttribute("BorderFillId", "0")),
            reader.Copy(new XElement("CELL", new XAttribute("BorderFill", "0"), new XAttribute("BorferFill", "0"), new XAttribute("BorderFillId", "0"))),
            "The absent BORDERFILL zero sentinel became an unknown reference.");
        Reject(() => reader.Copy(new XElement("P", new XAttribute("ParaShape", "0"))), "Unknown PARASHAPE reference 0.");

        var definedZero = Fixture();
        definedZero.Root!.Element("HEAD")!.Add(new XElement("BORDERFILL", new XAttribute("Id", "0"), new XElement("BRUSH", "실제 정의")));
        var zeroSource = new XElement("CELL", new XAttribute("BorderFill", "0"));
        var zeroCopy = new TemplateFormatting.ComparisonReader(definedZero).Copy(zeroSource);
        Equal(new XElement("CELL", new XAttribute("BorderFill", "<BORDERFILL><BRUSH>실제 정의</BRUSH></BORDERFILL>")), zeroCopy,
            "An existing BORDERFILL zero definition was incorrectly treated as the absent-definition sentinel.");
        Equal(TemplateFormatting.Copy(zeroSource, definedZero), zeroCopy, "Legacy and indexed BORDERFILL zero resolution differ.");

        var unusedDuplicates = Fixture();
        unusedDuplicates.Root!.Element("HEAD")!.Add(new XElement("CHARSHAPE", new XAttribute("Id", "99")),
            new XElement("CHARSHAPE", new XAttribute("Id", "99")));
        var duplicateReader = new TemplateFormatting.ComparisonReader(unusedDuplicates);
        Equal(ExpectedParagraph(), duplicateReader.Copy(Root(unusedDuplicates)), "Unused duplicate IDs were eagerly rejected.");
        Reject(() => duplicateReader.Copy(new XElement("TEXT", new XAttribute("CharShape", "99"))));
        Reject(() => TemplateFormatting.Copy(new XElement("TEXT", new XAttribute("CharShape", "99")), unusedDuplicates));

        var duplicateZero = Fixture();
        duplicateZero.Root!.Element("HEAD")!.Add(new XElement("BORDERFILL", new XAttribute("Id", "0")),
            new XElement("BORDERFILL", new XAttribute("Id", "0")));
        Reject(() => new TemplateFormatting.ComparisonReader(duplicateZero).Copy(new XElement("CELL", new XAttribute("BorderFill", "0"))));
        Reject(() => TemplateFormatting.Copy(new XElement("CELL", new XAttribute("BorderFill", "0")), duplicateZero));
    }

    private static void ReaderIsolationAndFreshScopes()
    {
        var first = Fixture();
        var second = Fixture();
        second.Descendants("CHARSHAPE").Single(e => (string?)e.Attribute("Id") == "5").SetAttributeValue("Height", "1800");
        var firstReader = new TemplateFormatting.ComparisonReader(first);
        var secondReader = new TemplateFormatting.ComparisonReader(second);
        var firstCopy = firstReader.Copy(Root(first));
        var secondCopy = secondReader.Copy(Root(second));
        Check(!XNode.DeepEquals(firstCopy, secondCopy), "Formatting caches leaked between documents with the same IDs.");
        Equal(ExpectedParagraph(), firstCopy, "The first document's definition was polluted by another reader.");
        Equal(TemplateFormatting.Copy(Root(second), second), secondCopy, "The second document did not retain its own definition.");
        Equal(secondCopy, new TemplateFormatting.ComparisonReader(second).Copy(Root(first)), "An explicit document did not override source.Document.");
        firstCopy.SetAttributeValue("ParaShape", "changed by caller");
        firstCopy.Descendants("CHAR").First().Value = "changed by caller";
        Equal(ExpectedParagraph(), firstReader.Copy(Root(first)), "Returned copies shared mutable cache state.");

        // A comparison reader owns one frozen inspection scope. Discard it before
        // changing definitions, as the rendering pipeline does between snapshots.
        first.Descendants("CHARSHAPE").Single(e => (string?)e.Attribute("Id") == "5").SetAttributeValue("Height", "1800");
        Equal(secondCopy, new TemplateFormatting.ComparisonReader(first).Copy(Root(first)), "A fresh reader reused stale definitions after document mutation.");
    }

    private static void StructuralComparisonPreservesInputsAndRejectsChanges()
    {
        var before = RichFixture();
        var after = new XDocument(before);
        after.Descendants("CHARSHAPE").Single(e => (string?)e.Attribute("Id") == "5").SetAttributeValue("Id", "15");
        foreach (var attribute in after.Descendants().Attributes("CharShape").Where(a => a.Value == "5").ToArray()) attribute.Value = "15";
        var firstRun = Root(after).Element("TEXT")!;
        var table = new XElement(firstRun.Element("TABLE")!);
        firstRun.ReplaceNodes(new XElement("CHAR", "앞 ", new XElement("LINEBREAK")));
        firstRun.AddAfterSelf(new XElement("TEXT", firstRun.Attributes(),
            new XElement("CHAR", " 중간 ", new XElement("TAB", new XAttribute("Width", "720")), " 끝"), table));
        var beforeXml = Xml(before);
        var afterXml = Xml(after);
        Check(TemplateRangeStructure.Equivalent([Root(before)], [Root(after)], before, after), "Equivalent renumbered formatting and TEXT segmentation were rejected.");
        TemplateRangeStructure.RequireOriginalStyleDefinitions(before, after);
        Check(Xml(before) == beforeXml && Xml(after) == afterXml, "Owned comparison canonicalization modified caller documents.");

        var leftExpanded = TemplateFormatting.Copy(Root(before), before);
        var rightExpanded = TemplateFormatting.Copy(Root(after), after);
        var leftXml = Xml(leftExpanded);
        var rightXml = Xml(rightExpanded);
        Check(TemplateRangeStructure.Equivalent([leftExpanded], [rightExpanded]), "Public and document-aware comparisons disagree.");
        _ = TemplateRangeStructure.DescribeDifference([Root(before)], [Root(after)], before, after);
        Check(Xml(leftExpanded) == leftXml && Xml(rightExpanded) == rightXml && Xml(before) == beforeXml && Xml(after) == afterXml,
            "Public comparison or difference reporting modified inputs.");
        var normalized = HancomPreviewWriter.NormalizeFigureMatrices(before);
        Check(Xml(before) == beforeXml && !ReferenceEquals(before, normalized), "NormalizeFigureMatrices no longer returns an independent copy.");
        Check(!normalized.Descendants("SCAMATRIX").Any() && normalized.Descendants("ROTMATRIX").Count() == 1 &&
            normalized.Descendants("SHAPECOMPONENT").All(e => e.Attribute("CurWidth") is null && e.Attribute("CurHeight") is null) &&
            normalized.Descendants("DRAWTEXT").All(e => e.Attribute("LastWidth") is null),
            "Figure normalization changed its established neutral-matrix and derived-layout policy.");

        foreach (var change in new[] { "text", "control", "geometry", "format", "style" })
        {
            var altered = new XDocument(after);
            switch (change)
            {
                case "text": altered.Descendants("CHAR").First().Value = "실제 변경"; break;
                case "control": altered.Descendants("TAB").Single().SetAttributeValue("Width", "721"); break;
                case "geometry": altered.Descendants("SIZE").Single().SetAttributeValue("Width", "1001"); break;
                case "format": altered.Descendants("CHARSHAPE").Single(e => (string?)e.Attribute("Id") == "15").SetAttributeValue("Height", "1201"); break;
                case "style": altered.Descendants("STYLE").Single().SetAttributeValue("Name", "다른 이름"); break;
            }
            if (change == "style") Reject(() => TemplateRangeStructure.RequireOriginalStyleDefinitions(before, altered));
            else Check(!TemplateRangeStructure.Equivalent([Root(before)], [Root(altered)], before, altered), "A real comparison change was ignored: " + change);
        }
    }

    private static XDocument Fixture() => XDocument.Parse("""
        <HWPML><HEAD xmlns:foreign="urn:foreign">
          <PARASHAPE Id="7" HeadingType="Number" Heading="3" TabDef="4" BorderFill="6" BorferFill="6" BorderFillId="6"><PARAMARGIN Left="100" /></PARASHAPE>
          <CHARSHAPE Id="5" Height="1200" BorderFill="6"><FONTID Hangul="2" /></CHARSHAPE>
          <CHARSHAPE Id="05" Height="1300" BorderFill="0" />
          <CHARSHAPE Height="1" /><CHARSHAPE Height="2" />
          <foreign:CHARSHAPE Id="5" Height="9999" />
          <BORDERFILL Id="6"><LEFTBORDER Type="Solid" Width="0.1mm" /></BORDERFILL>
          <TABDEF Id="4" AutoTabLeft="false"><TABITEM Pos="720" Type="Left" /></TABDEF>
          <NUMBERING Id="3"><PARAHEAD CharShape="5">^1.</PARAHEAD></NUMBERING>
          <BULLET Id="3" Char="●" CharShape="5" />
          <STYLE Id="0" Name="본문" ParaShape="7" CharShape="5" />
        </HEAD><BODY><SECTION><P ParaShape="7" Style="0" PageBreak="false" InstId="123"><TEXT CharShape="5"><CHAR>공백 한글 😀</CHAR></TEXT><TEXT CharShape="05"><CHAR>다른 서식</CHAR></TEXT></P></SECTION></BODY></HWPML>
        """);

    private static XDocument RichFixture()
    {
        var document = Fixture();
        var text = Root(document).Element("TEXT")!;
        text.ReplaceNodes(new XElement("CHAR", "앞 ", new XElement("LINEBREAK"), " 중간 ", new XElement("TAB", new XAttribute("Width", "720")), " 끝"),
            new XElement("TABLE", new XElement("SHAPEOBJECT", new XAttribute("InstId", "500"), new XAttribute("ZOrder", "9"),
                new XElement("SIZE", new XAttribute("Width", "1000"), new XAttribute("Height", "720"))),
                new XElement("SHAPECOMPONENT", new XAttribute("GroupLevel", "1"), new XAttribute("CurWidth", "1000"), new XAttribute("CurHeight", "720"),
                    new XElement("RENDERINGINFO", XElement.Parse("<SCAMATRIX E1=\"1\" E2=\"0\" E3=\"0\" E4=\"0\" E5=\"1\" E6=\"0\" />"),
                        XElement.Parse("<ROTMATRIX E1=\"1\" E2=\"0\" E3=\"1\" E4=\"0\" E5=\"1\" E6=\"0\" />"))),
                new XElement("DRAWTEXT", new XAttribute("LastWidth", "333"))));
        return document;
    }

    private static XElement ExpectedBorder() => XElement.Parse("<BORDERFILL><LEFTBORDER Type=\"Solid\" Width=\"0.1mm\" /></BORDERFILL>");
    private static XElement ExpectedCharacter() => new("CHARSHAPE", new XAttribute("Height", "1200"),
        new XAttribute("BorderFill", Xml(ExpectedBorder())), new XElement("FONTID", new XAttribute("Hangul", "2")));
    private static XElement ExpectedParagraph()
    {
        var paragraph = new XElement("PARASHAPE", new XAttribute("HeadingType", "Number"),
            new XAttribute("Heading", Xml(new XElement("NUMBERING", new XElement("PARAHEAD", new XAttribute("CharShape", Xml(ExpectedCharacter())), "^1.")))),
            new XAttribute("TabDef", Xml(XElement.Parse("<TABDEF AutoTabLeft=\"false\"><TABITEM Pos=\"720\" Type=\"Left\" /></TABDEF>"))),
            new XAttribute("BorderFill", Xml(ExpectedBorder())), new XAttribute("BorferFill", Xml(ExpectedBorder())), new XAttribute("BorderFillId", Xml(ExpectedBorder())),
            new XElement("PARAMARGIN", new XAttribute("Left", "100")));
        return new XElement("P", new XAttribute("ParaShape", Xml(paragraph)), new XAttribute("Style", "0"), new XAttribute("PageBreak", "false"), new XAttribute("InstId", "123"),
            new XElement("TEXT", new XAttribute("CharShape", Xml(ExpectedCharacter())), new XElement("CHAR", "공백 한글 😀")),
            new XElement("TEXT", new XAttribute("CharShape", "<CHARSHAPE Height=\"1300\" BorderFill=\"0\" />"), new XElement("CHAR", "다른 서식")));
    }

    private static XElement Root(XDocument document) => document.Descendants("SECTION").Single().Element("P")!;
    private static string Xml(XNode node) => node.ToString(SaveOptions.DisableFormatting);
    private static void Equal(XElement expected, XElement actual, string message) => Check(XNode.DeepEquals(expected, actual) && Xml(expected) == Xml(actual), message);
    private static void Check(bool value, string message) { if (!value) throw new Exception(message); }
    private static void Reject(Action action, string? expectedMessage = null)
    {
        try { action(); }
        catch (InvalidOperationException error)
        {
            if (expectedMessage is not null) Check(error.Message == expectedMessage, "Formatting rejection changed: " + error.Message);
            return;
        }
        throw new Exception("Expected formatting comparison rejection.");
    }
}
