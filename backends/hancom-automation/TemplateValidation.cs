// SPDX-License-Identifier: AGPL-3.0-only
using System.Xml.Linq;

namespace Md2Hwp.HancomIrPreview;

internal sealed record TemplateValidationResult(string Template, string IrVersion,
    string[] RequiredMetadata, bool RequiresRecognizedDate, bool SourceUnchanged);

internal static class TemplateValidation
{
    internal const string Usage = "usage: md2hwp-hancom validate-template [[--template] <template.hwp|template.hwpx>]";

    internal static string Parse(string[] args) => args switch
    {
        [] => Path.Combine(AppContext.BaseDirectory, "template.hwp"),
        [var path] when !path.StartsWith('-') => path,
        ["--template", var path] when !path.StartsWith('-') => path,
        _ => throw new ArgumentException(Usage)
    };

    internal static TemplateValidationResult Inspect(XDocument document, string path)
    {
        var metadata = TemplateMetadata.Inspect(document);
        InspectHeadingNumberScopes(metadata.Document);
        _ = TaggedTemplateBinding.Read(metadata.Document, path);
        return new(path, IrContract.Version, metadata.Keys, metadata.RequiresRecognizedDate, true);
    }

    internal static void InspectHeadingNumberScopes(XDocument document)
    {
        // Preserve the conversion path's private markers inside conditional
        // ranges, then expose any survivors for strict scope checking. These
        // lowerers validate every reference/heading prototype,
        // including both conditional branches and nested each.child containers.
        var references = TemplateCrossReferences.Lower(TemplateHeadingNumbers.Prepare(document));
        var headings = TemplateHeadingBlocks.Lower(references.Document);
        foreach (var paragraph in headings.Document.Descendants("P"))
        {
            var text = TemplateHeadingNumbers.PublicTags(TaggedTemplateBinding.DirectText(paragraph));
            if (!text.Contains("{{md2hwp:num:", StringComparison.Ordinal)) continue;
            var caption = paragraph.Parent?.Parent;
            var owner = caption?.Parent?.Parent;
            var captionRole = owner?.Name.LocalName switch
            {
                "PICTURE" => "figure",
                "TABLE" => "table",
                _ => null
            };
            if (caption?.Name.LocalName != "CAPTION" || captionRole is null ||
                !text.Contains(TaggedTemplateBinding.Tag("slot:" + captionRole + ".caption"), StringComparison.Ordinal))
                throw new InvalidDataException("Unbound heading number tag: use own/ancestor numbers in heading scopes, or num:heading1 in native figure/table captions and object-reference samples.");
            _ = TemplateHeadingNumbers.WithoutNumbers(text, 1);
        }
    }
}

internal static partial class HancomPreviewWriter
{
    internal static TemplateValidationResult ValidateTemplateFile(string path)
    {
        var source = ValidateTemplate(path);
        var hash = HashFile(source);
        EnsureInteractiveContext();
        EnsureNoExistingHwpProcess();
        var module = SecurityModuleRegistration.ReadAndValidate(Directory.GetCurrentDirectory());
        try
        {
            return WithHwp(module, hwp =>
            {
                Open(hwp, source, false);
                return TemplateValidation.Inspect(HwpMarkup.ReadDocument((object)hwp), source);
            });
        }
        finally
        {
            if (HashFile(source) != hash)
                throw new InvalidOperationException("Template changed during validation.");
        }
    }
}
