using System.Diagnostics;
using System.Text.Json;

namespace Md2Hwp.HancomIrPreview;

// Optional end-to-end timing, independent of Debug-only detailed profiling.
internal sealed class RenderTiming : IDisposable
{
    private readonly long started = Stopwatch.GetTimestamp();
    private bool completed;
    private bool disposed;

    internal static RenderTiming? Start() =>
        Environment.GetEnvironmentVariable("MD2HWP_TIMING") == "1" ? new() : null;

    internal void Complete() => completed = true;

    public void Dispose()
    {
        if (disposed) return;
        disposed = true;
        Console.Error.WriteLine("md2hwp-time: " + JsonSerializer.Serialize(new {
            Completed = completed,
            TotalMilliseconds = Math.Round(Stopwatch.GetElapsedTime(started).TotalMilliseconds, 3)
        }));
    }
}
