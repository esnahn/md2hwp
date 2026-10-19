// SPDX-License-Identifier: AGPL-3.0-only
namespace Md2Hwp.HancomIrPreview;

internal static class NativeDocumentFormat
{
    internal static string FromPath(string path) => Path.GetExtension(path).ToLowerInvariant() switch
    {
        ".hwp" => "HWP",
        ".hwpx" => "HWPX",
        _ => throw new ArgumentException($"Document must use .hwp or .hwpx: {path}")
    };

    internal static string TemporaryPath(string output, string prefix) =>
        Path.Combine(Path.GetDirectoryName(Path.GetFullPath(output))!,
            $".{prefix}-{Guid.NewGuid():N}{(FromPath(output) == "HWP" ? ".hwp" : ".hwpx")}");
}
