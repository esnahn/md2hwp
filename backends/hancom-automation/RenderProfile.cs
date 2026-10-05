using System.Diagnostics;
using System.Runtime.CompilerServices;
using System.Text.Json;
using System.Xml.Linq;

namespace Md2Hwp.HancomIrPreview;

// Opt-in developer timing. It never changes rendering or validation behavior.
internal sealed class RenderProfile : IDisposable
{
    [ThreadStatic] private static RenderProfile? current;
    private readonly RenderProfile? previous;
    private readonly long started = Stopwatch.GetTimestamp();
    private readonly Dictionary<string, Metric> metrics = new(StringComparer.Ordinal);
    private bool completed;
    private bool disposed;

    private RenderProfile() { previous = current; current = this; }

    internal static RenderProfile? Start() =>
        Environment.GetEnvironmentVariable("MD2HWP_PROFILE") == "1" ? new() : null;

    internal void Complete() => completed = true;

    internal static IDisposable Measure(string name) => current is { } profile
        ? new Measurement(profile, name) : EmptyMeasurement.Instance;

    internal static XDocument ReadDocument(object automation,
        [CallerMemberName] string caller = "", [CallerFilePath] string file = "")
    {
        dynamic hwp = automation;
        if (current is null) return HwpMarkup.Parse((string)hwp.GetTextFile("HWPML2X", ""));
        var location = Path.GetFileNameWithoutExtension(file) + "." + caller;
        string xml;
        using (Measure("hwpml.export/" + location))
            xml = (string)hwp.GetTextFile("HWPML2X", "");
        if (current is { } profile)
            profile.metrics["hwpml.export/" + location].CodeUnits += xml.Length;
        using (Measure("hwpml.parse/" + location))
            return HwpMarkup.Parse(xml);
    }

    public void Dispose()
    {
        if (disposed) return;
        disposed = true;
        current = previous;
        var entries = metrics.OrderByDescending(pair => pair.Value.Ticks)
            .Select(pair => new { Name = pair.Key, pair.Value.Calls,
                Milliseconds = Math.Round(pair.Value.Ticks * 1000.0 / Stopwatch.Frequency, 3),
                ExportedUtf16CodeUnits = pair.Value.CodeUnits }).ToArray();
        Console.Error.WriteLine("md2hwp-profile: " + JsonSerializer.Serialize(new {
            Completed = completed,
            TotalMilliseconds = Math.Round(Stopwatch.GetElapsedTime(started).TotalMilliseconds, 3),
            Metrics = entries
        }));
    }

    private sealed class Metric
    {
        internal long Calls;
        internal long Ticks;
        internal long CodeUnits;
    }
    private sealed class Measurement(RenderProfile profile, string name) : IDisposable
    {
        private readonly long started = Stopwatch.GetTimestamp();
        private bool disposed;
        public void Dispose()
        {
            if (disposed) return;
            disposed = true;
            if (!profile.metrics.TryGetValue(name, out var metric))
                profile.metrics.Add(name, metric = new());
            metric.Calls++;
            metric.Ticks += Stopwatch.GetTimestamp() - started;
        }
    }
    private sealed class EmptyMeasurement : IDisposable
    {
        internal static readonly EmptyMeasurement Instance = new();
        public void Dispose() { }
    }
}
