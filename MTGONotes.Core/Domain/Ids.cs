namespace MTGONotes.Core.Domain;

public readonly record struct EntityId
{
    private readonly string _value;

    private EntityId(string value) => _value = value;

    public static EntityId New() => new(Guid.CreateVersion7().ToString());

    public static EntityId Parse(string value)
    {
        if (!Guid.TryParse(value, out var parsed) || parsed.Version != 7)
        {
            throw new DomainException(RepoError.InvalidRequest);
        }

        return new EntityId(parsed.ToString());
    }

    public static bool TryParse(string value, out EntityId id)
    {
        try
        {
            id = Parse(value);
            return true;
        }
        catch (DomainException)
        {
            id = default;
            return false;
        }
    }

    public override string ToString() => _value ?? string.Empty;

    public string AsString() => ToString();
}

public readonly record struct IdempotencyKey
{
    private readonly EntityId _value;

    private IdempotencyKey(EntityId value) => _value = value;

    public static IdempotencyKey New() => new(EntityId.New());

    public static IdempotencyKey Parse(string value) => new(EntityId.Parse(value));

    public override string ToString() => _value.ToString();
}

public readonly record struct UtcMillis
{
    public UtcMillis(long value)
    {
        if (value < 0)
        {
            throw new DomainException(RepoError.InvalidRequest);
        }

        Value = value;
    }

    public long Value { get; }

    public static UtcMillis Now() =>
        new(DateTimeOffset.UtcNow.ToUnixTimeMilliseconds());
}

public readonly record struct Revision
{
    public static Revision Initial { get; } = new(1);

    public Revision(ulong value)
    {
        if (value == 0)
        {
            throw new DomainException(RepoError.InvalidRequest);
        }

        Value = value;
    }

    public ulong Value { get; }

    public Revision Next()
    {
        if (Value == ulong.MaxValue)
        {
            throw new DomainException(RepoError.RevisionConflict);
        }

        return new Revision(Value + 1);
    }
}
