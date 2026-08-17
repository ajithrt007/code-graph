using OrderSystem.Services;

namespace OrderSystem;

public class Program
{
    public static void Main()
    {
        var processor = new OrderProcessor(new OrderService(new OrderRepository()));
        processor.Process(42);
    }
}
