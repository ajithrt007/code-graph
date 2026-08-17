using System.Collections.Generic;

namespace OrderSystem.Services;

public record Order(int Id, string Customer);

public class OrderRepository
{
    private readonly Dictionary<int, Order> _store = new()
    {
        [42] = new Order(42, "Ada Lovelace"),
    };

    // Overloaded methods with identical names in the same type — the
    // analyzer must distinguish them by arity.
    public Order? FindById(int id) => _store.TryGetValue(id, out var o) ? o : null;

    public Order? FindById(int id, bool includeArchived) =>
        includeArchived ? _store.TryGetValue(id, out var o) ? o : null : null;
}
