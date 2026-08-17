namespace OrderSystem.Services;

public class OrderProcessor
{
    private readonly OrderService _service;

    public OrderProcessor(OrderService service) => _service = service;

    public void Process(int orderId)
    {
        var order = _service.GetOrder(orderId);
        if (order is null) return;
        _service.Print(order);
    }
}
