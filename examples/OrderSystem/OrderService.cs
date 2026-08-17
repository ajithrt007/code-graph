using System;

namespace OrderSystem.Services;

public class OrderService
{
    private readonly OrderRepository _repository;

    public OrderService(OrderRepository repository) => _repository = repository;

    public Order? GetOrder(int id) => _repository.FindById(id);

    // Demonstrates a method that *also* lives on another type with the
    // same name. Without semantic resolution the analyzer would conflate
    // these two `Print` calls.
    public void Print(Order order)
    {
        Console.WriteLine($"Order #{order.Id} for {order.Customer}");
        PrintFooter();
    }

    private void PrintFooter()
    {
        Console.WriteLine("----");
    }
}
