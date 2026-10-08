using System.Text.Json;
using System.Xml.Linq;
using Md2Hwp.HancomIrPreview;

internal static class TableContractTests
{
    internal static void Run()
    {
        var source = new TemplateSource(Paragraph("source"), "source", "", "");
        var profile = InvestigationTemplateProfile.FromTaggedTemplate(
            typeof(IrPreviewPlan).Assembly.Location, [], "reset", 100, 6, 0,
            new ProfileCaptionSelector("caption", "[", "]caption", "caption"), source, source,
            new TemplateListPrototype("bullet", 0, new XElement("BULLET")),
            new TemplateListPrototype("ordered", 0, new XElement("NUMBERING")));
        IrPreviewPlan Read(string json, params PreviewOperation[] prefix)
        {
            using var input = JsonDocument.Parse(json);
            var builder = new PlanBuilder("fixture.ir.json", Directory.GetCurrentDirectory(), profile);
            foreach (var operation in prefix)
                builder.AddBlock(JsonDocument.Parse($$"""{"type":"heading","level":1,"id":"{{operation.HeadingId}}","inlines":[{"type":"text","value":"Target"}]}""").RootElement, "/blocks/0");
            builder.AddBlock(input.RootElement, "/blocks/1");
            return builder.Build(prefix.Length + 1);
        }
        const string text = """{"type":"text","value":"내용"}""";
        const string note = """{"type":"footnote","blocks":[{"type":"paragraph","inlines":[{"type":"text","value":"각주"}]}]}""";
        const string reference = """{"type":"cross_reference","kind":"heading_number","target":"대상"}""";
        var json = $$"""{"type":"table","columns":["left","right"],"header":[[{{text}}],[]],"rows":[[[{{note}},{{reference}}],[{"type":"strong","inlines":[{{text}}]}]]],"caption":[{{text}}],"source":[{"prefix":"출처","inlines":[{{text}}]}]}""";
        var plan = Read(json, new PreviewOperation("text", "target", [], HeadingId: "대상"));
        var operation = plan.Operations.Last();
        Check(plan.Summary.TableOperations == 1 && operation.Kind == "table" && operation.Table!.Rows.Count == 1,
            "Table operations or their row model were lost.");
        Check(operation.FormattedLines is not null && operation.FormattedLines.Count == 1 &&
            string.Concat(operation.FormattedLines[0].Select(run => run.Text)) == operation.Lines.Single(),
            "Table insertion marker does not carry the required paragraph formatting.");
        Check(operation.Table!.Header[1].Lines.Single().Text.Length == 0, "Empty table cells were rejected or lost.");
        Check(FigureReferenceContract.ReadRuns(plan.Operations).Count(run => run.Footnote is not null) == 1 &&
            FigureReferenceContract.ReadRuns(plan.Operations).Count(run => run.CrossReference is not null) == 1,
            "Native footnote/reference visitors do not traverse table cells.");
        Check(Read("""{"type":"table","columns":["default"],"header":[[]],"rows":[],"caption":null,"source":null}""")
            .Operations.Single().Table is { Caption: null, Source: null }, "Null optional table fields were rejected.");
        foreach (var invalid in new[]
        {
            """{"type":"table","columns":[],"header":[],"rows":[]}""",
            """{"type":"table","columns":["auto"],"header":[[]],"rows":[]}""",
            """{"type":"table","columns":["default"],"header":[],"rows":[]}""",
            """{"type":"table","columns":["default"],"header":[[]],"rows":[[]]}""",
            """{"type":"table","columns":["default"],"header":[[]],"rows":[],"caption":[]}""",
            """{"type":"table","columns":["default"],"header":[[]],"rows":[],"source":[]}""",
            """{"type":"table","columns":["default"],"header":[[]],"rows":[],"width":10}""",
            $$"""{"type":"table","columns":["default"],"header":[[]],"rows":[],"caption":[{{note}}]}""",
            $$"""{"type":"table","columns":["default"],"header":[[]],"rows":[],"source":[{"prefix":"출처","inlines":[{{reference}}]}]}""",
            $$"""{"type":"table","columns":["default"],"header":[[{{reference}}]],"rows":[]}""",
        }) Reject(() => Read(invalid));

        var original = Fixture();
        var before = original.ToString();
        var lower = TemplateTables.Lower(original);
        Check(original.ToString() == before, "Table lowering mutated its source.");
        Check(AuriMinimalBoxPrototype.RootParagraphs(lower.Document).Select(TaggedTemplateBinding.DirectText)
            .SequenceEqual(new[] { "before", Tag("begin:template"), "keep", Tag("end:template"), "after" }),
            "Table lowering removed unrelated roots.");
        Check(lower.Layout.WidthHwpUnits == 14400, "Millimeter table width did not become the requested HWP units.");

        var cellHeader = Inline("""[{"type":"text","value":"이름"}]""");
        var cellBody = Inline("""[{"type":"text","value":"첫"},{"type":"line_break"},{"type":"emph","inlines":[{"type":"text","value":"둘"}]}]""");
        var content = new PreviewTable(["default", "right"], [cellHeader, Inline("[]")],
            [[cellBody, Inline("[" + note + "," + reference + "]")]],
            Inline("[" + text + "]"), Inline("[" + text + "]"));
        var tableOperation = new PreviewOperation("table", "table", ["MARKER"], ParagraphStyle: "body", Heading1Number: 3, Table: content);
        var tablePlan = new IrPreviewPlan("fixture", "fixture", new(1, 0, 0, 0, 0, 1), [tableOperation], []);
        var destination = Destination(Paragraph("MARKER"));
        var attached = lower.Layout.Attach(destination, tablePlan, 0, (_, _, _) => [4000, 10400]);
        var table = attached.Descendants("TABLE").Single();
        Check((int?)table.Attribute("RowCount") == 3 && (int?)table.Attribute("ColCount") == 2 &&
            table.Elements("ROW").Take(2).All(row => row.Elements("CELL").Count() == 2), "Generated table has the wrong row/column geometry.");
        Check(table.Elements("ROW").Last().Elements("CELL").Single().Attribute("ColSpan")!.Value == "2" &&
            table.Elements("ROW").Last().Descendants("P").Single().Value == "출처: 내용",
            "Source row was not merged or its literal template header was lost.");
        Check(table.Elements("ROW").First().Elements("CELL").All(cell => (string?)cell.Attribute("Header") == "true") &&
            table.Elements("ROW").Skip(1).SelectMany(row => row.Elements("CELL")).All(cell => (string?)cell.Attribute("Header") == "false"),
            "Only header cells should repeat on following pages.");
        Check(table.Elements("SHAPEOBJECT").Elements("SIZE").Single().Attribute("Width")!.Value == "14400" &&
            table.Descendants("CELL").Select(cell => (string)cell.Attribute("Height")!).All(height => height == "1"),
            "Generated geometry inherited sample dimensions.");
        Check(table.Descendants("CAPTION").Single().Value == "[표 -] 내용" &&
            table.Descendants("AUTONUM").Single().Attribute("NumberType")!.Value == "Table",
            "Native table caption or automatic numbering was lost.");
        Check(table.Descendants("LINEBREAK").Count() == 1 && attached.Descendants("CHARSHAPE").Any(shape => shape.Element("ITALIC") is not null),
            "Table cell rich text or line breaks were lost.");
        Check(table.Descendants("P").Select(paragraph => (string)paragraph.Attribute("InstId")!).Distinct().Count() == table.Descendants("P").Count(),
            "Cloned table cell paragraph instance IDs collide.");
        var rightParagraph = table.Elements("ROW").First().Elements("CELL").Last().Descendants("P").Single();
        Check((string?)attached.Descendants("PARASHAPE").Single(shape => (string?)shape.Attribute("Id") ==
            (string?)rightParagraph.Attribute("ParaShape")).Attribute("Align") == "Right", "Explicit manuscript alignment was not applied.");
        Check(lower.Layout.GeneratedTableInstances.Count == 1, "Generated table native identity was not recorded.");

        var without = tableOperation with { Table = content with { Caption = null, Source = null } };
        var absent = lower.Layout.Attach(Destination(Paragraph("MARKER")), tablePlan with { Operations = [without] }, 0, (_, _, _) => [4000, 10400]);
        Check(!absent.Descendants("CAPTION").Any() && absent.Descendants("ROW").Count() == 2,
            "Absent caption/source left a native caption or empty source row.");
        Reject(() => lower.Layout.Attach(Destination(Paragraph("MARKER")), tablePlan, 0, (_, _, _) => [4000, 10000]));
        Reject(() => lower.Layout.Attach(Destination(Paragraph("MARKER")), tablePlan, 0, (_, _, _) => [0, 14400]));
        Reject(() => lower.Layout.Attach(Destination(Paragraph("wrong")), tablePlan, 0, (_, _, _) => [4000, 10400]));
        // Paths capture generated tables only; widths and formatting stay strict.
        attached = lower.Layout.Attach(destination, tablePlan, 0, (_, _, _) => [4000, 10400]);
        lower.Layout.RecordLayout(attached);
        var reflow = new XDocument(attached);
        foreach (var height in reflow.Descendants("TABLE").Descendants().Attributes("Height")) height.Value = "2000";
        var widthAttribute = reflow.Descendants("CELL").First().Attribute("Width")!; widthAttribute.Value = "9999";
        lower.Layout.NormalizeLayout(attached, reflow);
        Check(reflow.Descendants("CELL").All(cell => (string?)cell.Attribute("Height") == "1") && widthAttribute.Value == "9999",
            "Table reflow normalization altered widths or failed to normalize generated heights.");

        foreach (var invalid in new[] { "missing", "duplicate", "rows", "columns", "merged", "source-span", "old-prototype", "right-slot", "right-border", "address", "source-border", "caption", "native", "outside", "width", "orphan", "list-control", "caption-list-control", "shadow", "threed", "malformed" })
        {
            var fixture = Fixture();
            var section = fixture.Descendants("SECTION").Single();
            var sample = fixture.Descendants("TABLE").Single();
            switch (invalid)
            {
                case "missing": section.Elements("P").First(p => TaggedTemplateBinding.DirectText(p) == Tag("begin:table")).Remove(); break;
                case "duplicate": section.Add(Paragraph(Tag("begin:table"))); break;
                case "rows": sample.Elements("ROW").Last().Remove(); break;
                case "columns": sample.SetAttributeValue("ColCount", 1); break;
                case "merged": sample.Descendants("CELL").First().SetAttributeValue("ColSpan", 2); break;
                case "source-span": sample.Descendants("CELL").Last().SetAttributeValue("ColSpan", 1); break;
                case "old-prototype":
                    foreach (var row in sample.Elements("ROW").Take(2)) row.Elements("CELL").Last().Remove();
                    sample.SetAttributeValue("ColCount", 1); sample.Descendants("CELL").Last().SetAttributeValue("ColSpan", 1); break;
                case "right-slot": sample.Elements("ROW").First().Elements("CELL").Last().Descendants("CHAR").Single().Value = "wrong"; break;
                case "right-border": sample.Elements("ROW").First().Elements("CELL").Last().SetAttributeValue("BorderFill", 999); break;
                case "address": sample.Elements("ROW").First().Elements("CELL").Last().SetAttributeValue("ColAddr", 0); break;
                case "source-border": sample.Descendants("CELL").Last().SetAttributeValue("BorderFill", 1); break;
                case "caption": sample.Descendants("CAPTION").Single().Remove(); break;
                case "native": sample.Elements("ROW").First().Descendants("TEXT").First().Add(new XElement("FOOTNOTE")); break;
                case "outside": section.Elements("P").First(p => TaggedTemplateBinding.DirectText(p) == Tag("end:template")).Remove(); break;
                case "width": section.Elements("P").First(p => TaggedTemplateBinding.DirectText(p).Contains("table.width-mm", StringComparison.Ordinal)).Element("TEXT")!.Element("CHAR")!.Value = Tag("table.width-mm:0"); break;
                case "orphan": section.Add(Paragraph(TemplateTables.CaptionSlot)); break;
                case "list-control": sample.Elements("ROW").First().Descendants("PARALIST").First().Add(new XElement("SECDEF")); break;
                case "caption-list-control": sample.Descendants("CAPTION").Single().Element("PARALIST")!.Add(new XElement("COLDEF")); break;
                case "shadow": fixture.Descendants("BORDERFILL").Single(border => (string?)border.Attribute("Id") == "2").SetAttributeValue("Shadow", "true"); break;
                case "threed": fixture.Descendants("BORDERFILL").Single(border => (string?)border.Attribute("Id") == "2").SetAttributeValue("ThreeD", "true"); break;
                case "malformed": sample.Descendants("CAPTION").Single().Descendants("CHAR").Single().Value += "{{md2hwp:bad"; break;
            }
            Reject(() => TemplateTables.Lower(fixture));
        }
        var counters = new XDocument(new XElement("HWPML", new XElement("DOCSETTING", new XElement("BEGINNUMBER", new XAttribute("Table", 5))),
            new XElement("BODY", new XElement("SECTION", new XElement("AUTONUM", new XAttribute("NumberType", "Table"), new XAttribute("Number", 1)),
                new XElement("NEWNUM", new XAttribute("NumberType", "Table"), new XAttribute("Number", 2)),
                new XElement("AUTONUM", new XAttribute("NumberType", "Table"), new XAttribute("Number", 1))))));
        TemplateTables.RecalculateTableNumbers(counters);
        Check(counters.Descendants("AUTONUM").Select(number => (int)number.Attribute("Number")!).SequenceEqual(new[] { 5, 2 }),
            "Existing native table start/restart controls were not respected.");
        var sectionCounter = new XDocument(counters);
        sectionCounter.Descendants("SECTION").Single().AddFirst(new XElement("SECDEF",
            new XElement("STARTNUMBER", new XAttribute("Table", 7))));
        TemplateTables.RecalculateTableNumbers(sectionCounter);
        Check(sectionCounter.Descendants("AUTONUM").Select(number => (int)number.Attribute("Number")!).SequenceEqual(new[] { 7, 2 }),
            "The section's native table start number was not respected.");
        sectionCounter.Descendants("STARTNUMBER").Single().SetAttributeValue("Table", 0);
        TemplateTables.RecalculateTableNumbers(sectionCounter);
        Check(sectionCounter.Descendants("AUTONUM").Select(number => (int)number.Attribute("Number")!).SequenceEqual(new[] { 5, 2 }),
            "Zero section start should continue the table counter.");
        TestHeaderPagination(tableOperation);
        TestVerticalBorders();
        TestSharedSourceBorder();
        Console.WriteLine("Table IR, sample rows, source omission, rich cells, native captions, geometry and local header pagination contracts passed.");
    }

    private static void TestSharedSourceBorder()
    {
        var fixture = Fixture();
        var bodyBorder = fixture.Descendants("BORDERFILL").Single(b => (string?)b.Attribute("Id") == "1");
        bodyBorder.Add(new XElement("BOTTOMBORDER", new XAttribute("Type", "Solid"), new XAttribute("Width", "0.12mm"), new XAttribute("Color", "255")));
        var sourceBorder = fixture.Descendants("BORDERFILL").Single(b => (string?)b.Attribute("Id") == "2");
        sourceBorder.Add(new XElement("TOPBORDER", bodyBorder.Element("BOTTOMBORDER")!.Attributes()));
        var original = fixture.ToString();
        var layout = TemplateTables.Lower(fixture).Layout;
        var cells = new[] { Inline("[]"), Inline("[]") };
        var content = new PreviewTable(["default", "default"], cells, [cells], Source: Inline("[{\"type\":\"text\",\"value\":\"자료\"}]"));
        var operation = new PreviewOperation("table", "table", ["MARKER"], ParagraphStyle: "body", Table: content);
        var plan = new IrPreviewPlan("fixture", "fixture", new(1, 0, 0, 0, 0, 1), [operation], []);
        var attached = layout.Attach(Destination(Paragraph("MARKER")), plan, 0, (_, _, _) => [7200, 7200]);
        XElement Border(XDocument doc, XElement cell) => doc.Descendants("BORDERFILL").Single(b =>
            (string?)b.Attribute("Id") == (string?)cell.Attribute("BorderFill"));
        var table = attached.Descendants("TABLE").Single();
        Check(XNode.DeepEquals(Border(attached, table.Elements("ROW").Last().Elements("CELL").Single()).Element("TOPBORDER"), sourceBorder.Element("TOPBORDER")),
            "The shared source boundary was removed or changed.");
        Check(fixture.ToString() == original, "Shared source border attachment changed its source template.");
        foreach (var attribute in new[] { "Width", "Color", "Type" })
        {
            var mismatch = new XDocument(fixture);
            mismatch.Descendants("BORDERFILL").Single(b => (string?)b.Attribute("Id") == "2").Element("TOPBORDER")!
                .SetAttributeValue(attribute, attribute == "Type" ? "DoubleSlim" : "999");
            Reject(() => TemplateTables.Lower(mismatch));
        }
        var headerBorder = new XElement(bodyBorder); headerBorder.SetAttributeValue("Id", 3);
        headerBorder.Element("BOTTOMBORDER")!.SetAttributeValue("Width", "0.3mm");
        bodyBorder.Parent!.Add(headerBorder); bodyBorder.Parent.SetAttributeValue("Count", 3);
        foreach (var cell in fixture.Descendants("TABLE").Single().Elements("ROW").First().Elements("CELL"))
            cell.SetAttributeValue("BorderFill", 3);
        var headerLayout = TemplateTables.Lower(fixture).Layout;
        var headerOnly = operation with { Table = content with { Rows = [] } };
        var headerOutput = headerLayout.Attach(Destination(Paragraph("MARKER")), plan with { Operations = [headerOnly] }, 0, (_, _, _) => [7200, 7200]);
        var sourceCell = headerOutput.Descendants("TABLE").Single().Elements("ROW").Last().Elements("CELL").Single();
        Check((string?)Border(headerOutput, sourceCell).Element("TOPBORDER")?.Attribute("Width") == "0.3mm",
            "A header-only table kept the body's border instead of the actual preceding header boundary.");
        var absent = operation with { Table = content with { Source = null } };
        var absentOutput = layout.Attach(Destination(Paragraph("MARKER")), plan with { Operations = [absent] }, 0, (_, _, _) => [7200, 7200]);
        var lastBody = absentOutput.Descendants("TABLE").Single().Elements("ROW").Last().Elements("CELL").First();
        Check((string?)Border(absentOutput, lastBody).Element("BOTTOMBORDER")?.Attribute("Width") == "0.12mm",
            "Omitting the source also removed the body bottom border.");
    }

    private static void TestVerticalBorders()
    {
        var fixture = Fixture();
        var left = fixture.Descendants("BORDERFILL").Single(border => (string?)border.Attribute("Id") == "1");
        left.Element("LEFTBORDER")!.SetAttributeValue("Width", "0.7mm");
        left.Element("RIGHTBORDER")!.SetAttributeValue("Width", "0.1mm");
        left.Add(new XElement("TOPBORDER", new XAttribute("Type", "Solid"), new XAttribute("Width", "0.3mm")),
            new XElement("BOTTOMBORDER", new XAttribute("Type", "Solid"), new XAttribute("Width", "0.5mm")));
        var right = new XElement(left); right.SetAttributeValue("Id", 3);
        right.Element("LEFTBORDER")!.SetAttributeValue("Width", "0.2mm");
        right.Element("RIGHTBORDER")!.SetAttributeValue("Width", "0.8mm");
        left.Parent!.Add(right); left.Parent.SetAttributeValue("Count", 3);
        foreach (var row in fixture.Descendants("TABLE").Single().Elements("ROW").Take(2))
            row.Elements("CELL").Last().SetAttributeValue("BorderFill", 3);
        var original = fixture.ToString();
        var layout = TemplateTables.Lower(fixture).Layout;
        foreach (var columns in new[] { 1, 2, 4 })
        {
            var cells = Enumerable.Repeat(Inline("[{\"type\":\"text\",\"value\":\"cell\"}]"), columns).ToArray();
            var content = new PreviewTable(Enumerable.Repeat("default", columns).ToArray(), cells, [cells]);
            var operation = new PreviewOperation("table", "table", ["MARKER"], ParagraphStyle: "body", Table: content);
            var plan = new IrPreviewPlan("fixture", "fixture", new(1, 0, 0, 0, 0, 1), [operation], []);
            var attached = layout.Attach(Destination(Paragraph("MARKER")), plan, 0,
                (_, _, _) => Enumerable.Repeat(14400 / columns, columns).ToArray());
            foreach (var row in attached.Descendants("TABLE").Single().Elements("ROW"))
            foreach (var (cell, column) in row.Elements("CELL").Select((cell, column) => (cell, column)))
            {
                var border = attached.Descendants("BORDERFILL").Single(value =>
                    (string?)value.Attribute("Id") == (string?)cell.Attribute("BorderFill"));
                Check((string?)border.Element("LEFTBORDER")?.Attribute("Width") == (column == 0 ? "0.7mm" : "0.2mm") &&
                    (string?)border.Element("RIGHTBORDER")?.Attribute("Width") == (column == columns - 1 ? "0.8mm" : "0.1mm"),
                    "Outer/internal vertical borders were lost in a one-, two- or many-column table.");
                Check((string?)border.Element("TOPBORDER")?.Attribute("Width") == "0.3mm" &&
                    (string?)border.Element("BOTTOMBORDER")?.Attribute("Width") == "0.5mm",
                    "Composing vertical borders changed horizontal borders.");
            }
        }
        Check(fixture.ToString() == original, "Vertical border composition mutated the source template.");
    }

    private static void TestHeaderPagination(PreviewOperation operation)
    {
        var fixture = Fixture();
        var fixtureBefore = fixture.ToString();
        var layout = TemplateTables.Lower(fixture).Layout;
        var staticParagraph = new XElement(fixture.Descendants("TABLE").Single().Ancestors("P").Last());
        var destination = Destination(staticParagraph, Paragraph("MARKER"), Paragraph("MARKER"), Paragraph("after"));
        var plan = new IrPreviewPlan("fixture", "fixture", new(2, 0, 0, 0, 0, 2), [operation, operation], []);
        XDocument Generate() => layout.Attach(destination, plan, 1, (_, _, _) => [4000, 10400]);
        var generated = Generate();
        Check(layout.GeneratedTableBodyInstances.Count == 2, "Body-bearing generated tables were not recorded.");
        var expected = new XDocument(generated);
        var expectedRoot = AuriMinimalBoxPrototype.RootParagraphs(expected)[1];
        expectedRoot.SetAttributeValue("PageBreak", "true"); expectedRoot.SetAttributeValue("ColumnBreak", "false");
        var calls = new List<int>();
        var imports = 0;
        var corrected = NativeTablePagination.Correct(generated, NativeTablePagination.ResolveAnchors(generated, layout.GeneratedTableBodyInstances), anchor =>
        {
            Check(anchor.Columns == 2, "Native pagination did not use the manuscript's column count.");
            calls.Add(anchor.RootParagraph);
            return anchor.RootParagraph == 1
                ? new NativeTablePagination.StartPages(1, imports == 0 ? 2 : 1)
                : new NativeTablePagination.StartPages(3, imports == 0 ? 4 : 3);
        }, document =>
        {
            imports++;
            Check(ReferenceEquals(document, generated), "Native reflow did not import the corrected working document.");
        });
        Check(corrected == 1 && imports == 1 && calls.SequenceEqual(new[] { 1, 1, 2 }),
            "Later table pagination was not re-read after a preceding correction.");
        Check(XNode.DeepEquals(generated, expected),
            "Orphan-header correction changed table formats, native captions, cells, surrounding content or the static table.");
        Check(fixture.ToString() == fixtureBefore, "Native pagination mutated the source template.");

        var samePage = Generate();
        var sameBefore = samePage.ToString();
        Check(NativeTablePagination.Correct(samePage, NativeTablePagination.ResolveAnchors(samePage, layout.GeneratedTableBodyInstances),
            _ => new(2, 2), _ => throw new Exception("Unneeded table reimport.")) == 0 && samePage.ToString() == sameBefore,
            "Tables with a first body row on the header page acquired a forced page break.");
        var renumbered = Generate();
        var nativeAnchors = NativeTablePagination.ResolveAnchors(renumbered, layout.GeneratedTableBodyInstances);
        foreach (var shape in renumbered.Descendants("SHAPEOBJECT")) shape.SetAttributeValue("InstId", Guid.NewGuid().ToString("N"));
        var renumberedBefore = renumbered.ToString();
        Check(NativeTablePagination.Correct(renumbered, nativeAnchors, _ => new(2, 2),
            _ => throw new Exception("Unneeded renumbered table import.")) == 0 && renumbered.ToString() == renumberedBefore,
            "Native reassignment of drawing identities invalidated verified table coordinates.");
        var nextColumn = Generate();
        var columnImports = 0;
        var columnAnchors = NativeTablePagination.ResolveAnchors(nextColumn, layout.GeneratedTableBodyInstances);
        Check(NativeTablePagination.Correct(nextColumn, columnAnchors, _ => columnImports == 0 ? new(2, 2, 1, 2) : new(3, 3),
            _ => columnImports++) == 1 && columnImports == 1,
            "A first body row in a later text column was mistaken for the header's position.");
        NativeTablePagination.Verify(columnAnchors, _ => new(3, 3));
        RejectPagination(() => NativeTablePagination.Verify(columnAnchors, _ => new(3, 4)));
        RejectPagination(() => NativeTablePagination.Verify(columnAnchors, _ => new(3, 3, 1, 2)));
        var firstRoot = AuriMinimalBoxPrototype.RootParagraphs(samePage)[1];
        firstRoot.SetAttributeValue("PageBreak", "true");
        RejectPagination(() => NativeTablePagination.Correct(samePage, NativeTablePagination.ResolveAnchors(samePage, layout.GeneratedTableBodyInstances),
            _ => new(2, 3), _ => throw new Exception("An already forced table was imported again.")));

        var cannotFit = Generate();
        var failedImports = 0;
        RejectPagination(() => NativeTablePagination.Correct(cannotFit, NativeTablePagination.ResolveAnchors(cannotFit, layout.GeneratedTableBodyInstances),
            _ => new(2, 3), _ => failedImports++));
        Check(failedImports == 1, "Pagination failure caused repeated forced breaks instead of a bounded failure.");
        foreach (var invalid in new[] { new NativeTablePagination.StartPages(0, 2), new NativeTablePagination.StartPages(3, 2),
            new NativeTablePagination.StartPages(2, 2, 2, 1), new NativeTablePagination.StartPages(2, 2, 0, 1) })
        {
            var invalidDocument = Generate();
            var before = invalidDocument.ToString();
            RejectPagination(() => NativeTablePagination.Correct(invalidDocument, NativeTablePagination.ResolveAnchors(invalidDocument, layout.GeneratedTableBodyInstances),
                _ => invalid, _ => throw new Exception("Invalid native pages were imported.")));
            Check(invalidDocument.ToString() == before, "Invalid page positions changed the working document.");
        }
        RejectPagination(() => NativeTablePagination.ResolveAnchors(Generate(), ["missing-instance"]));
        RejectPagination(() => NativeTablePagination.Correct(Generate(), [new(999, 2)],
            _ => throw new Exception("Missing generated table was read."), _ => throw new Exception("Missing generated table was imported.")));

        var headerOnly = operation with { Table = operation.Table! with { Rows = [] } };
        var headerPlan = plan with { Operations = [headerOnly, headerOnly] };
        var headerDocument = layout.Attach(destination, headerPlan, 1, (_, _, _) => [4000, 10400]);
        Check(layout.GeneratedTableInstances.Count == 2 && layout.GeneratedTableBodyInstances.Count == 0,
            "A source row was mistaken for a manuscript body row.");
        Check(NativeTablePagination.Correct(headerDocument, NativeTablePagination.ResolveAnchors(headerDocument, layout.GeneratedTableBodyInstances),
            _ => throw new Exception("Header-only table was measured."), _ => throw new Exception("Header-only table was imported.")) == 0,
            "Header-only tables acquired a forced page break.");
    }

    private static void RejectPagination(Action action)
    { try { action(); } catch (InvalidOperationException) { return; } throw new Exception("Expected native table pagination rejection."); }

    private static PreviewInlineContent Inline(string json)
    { using var input = JsonDocument.Parse(json); return InlineText.Read(input.RootElement, "/inlines"); }

    private static string Tag(string token) => TaggedTemplateBinding.Tag(token);
    private static XElement Paragraph(string value) => new("P", new XAttribute("ParaShape", 0), new XAttribute("Style", 0),
        new XAttribute("InstId", Guid.NewGuid().GetHashCode().ToString()), new XElement("TEXT", new XAttribute("CharShape", 0), new XElement("CHAR", value)));

    private static XDocument Destination(params XElement[] roots) => new(new XElement("HWPML",
        new XElement("HEAD",
            new XElement("PARASHAPELIST", new XAttribute("Count", 1), new XElement("PARASHAPE", new XAttribute("Id", 0), new XAttribute("Align", "Center"))),
            new XElement("CHARSHAPELIST", new XAttribute("Count", 1), new XElement("CHARSHAPE", new XAttribute("Id", 0), new XAttribute("Height", 1000))),
            new XElement("BORDERFILLLIST", new XAttribute("Count", 2),
                new XElement("BORDERFILL", new XAttribute("Id", 1), new XElement("LEFTBORDER", new XAttribute("Type", "Solid")), new XElement("RIGHTBORDER", new XAttribute("Type", "Solid"))),
                new XElement("BORDERFILL", new XAttribute("Id", 2), new XElement("LEFTBORDER", new XAttribute("Type", "None"))))),
        new XElement("BODY", new XElement("SECTION", roots))));

    private static XDocument Fixture()
    {
        XElement Row(string slot, int index) => new("ROW", Enumerable.Range(0, index == 2 ? 1 : 2).Select(column => new XElement("CELL",
            new XAttribute("ColAddr", column), new XAttribute("RowAddr", index), new XAttribute("ColSpan", index == 2 ? 2 : 1), new XAttribute("RowSpan", 1),
            new XAttribute("Width", 3000), new XAttribute("Height", 9000), new XAttribute("BorderFill", index == 2 ? 2 : 1),
            new XElement("CELLMARGIN", new XAttribute("Left", 100), new XAttribute("Right", 100)),
            new XElement("PARALIST", Paragraph(index == 2 ? "출처: " + slot : slot)))));
        var caption = Paragraph("[표 -] " + TemplateTables.CaptionSlot);
        caption.Element("TEXT")!.AddFirst(new XElement("AUTONUM", new XAttribute("Number", 1), new XAttribute("NumberType", "Table"),
            new XElement("AUTONUMFORMAT", new XAttribute("Type", "Digit"))));
        var root = Paragraph("");
        root.Element("TEXT")!.AddFirst(new XElement("TABLE", new XAttribute("RowCount", 3), new XAttribute("ColCount", 2), new XAttribute("RepeatHeader", "true"),
            new XElement("SHAPEOBJECT", new XAttribute("InstId", 100), new XAttribute("NumberingType", "Table"),
                new XElement("SIZE", new XAttribute("Width", 3000), new XAttribute("Height", 27000)),
                new XElement("CAPTION", new XAttribute("Side", "Top"), new XAttribute("Gap", 300), new XElement("PARALIST", caption))),
            Row(TemplateTables.HeaderSlot, 0), Row(TemplateTables.ContentSlot, 1), Row(TemplateTables.SourceSlot, 2)));
        return Destination(Paragraph("before"), Paragraph(Tag("begin:template")), Paragraph("keep"),
            Paragraph(Tag("table.width-mm:50.8")), Paragraph(Tag("begin:table")), root, Paragraph(Tag("end:table")),
            Paragraph(Tag("end:template")), Paragraph("after"));
    }

    private static void Check(bool value, string message) { if (!value) throw new Exception(message); }
    private static void Reject(Action action) { try { action(); } catch (InvalidDataException) { return; } throw new Exception("Expected table contract rejection."); }
}
