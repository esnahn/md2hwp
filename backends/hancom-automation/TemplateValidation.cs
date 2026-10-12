// SPDX-License-Identifier: AGPL-3.0-only
using System.Xml.Linq;

namespace Md2Hwp.HancomIrPreview;

internal sealed record TemplateValidationResult(string Template, string IrVersion,
    string[] RequiredMetadata, bool RequiresRecognizedDate, bool SourceUnchanged);

internal static class TemplateValidation
{
    internal const string Usage = "usage: md2hwp-backend validate-template [[--template] <template.hwp>]";

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
        _ = TaggedTemplateBinding.Read(metadata.Document, path);
        return new(path, IrContract.Version, metadata.Keys, metadata.RequiresRecognizedDate, true);
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
