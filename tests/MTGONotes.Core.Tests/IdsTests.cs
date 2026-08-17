using MTGONotes.Core.Domain;

namespace MTGONotes.Core.Tests;

public sealed class IdsTests
{
    [Fact]
    public void Domain_identifiers_are_uuid_v7_strings()
    {
        var id = EntityId.New();
        Assert.True(Guid.TryParse(id.AsString(), out var parsed));
        Assert.Equal(7, parsed.Version);
        Assert.Equal(36, id.AsString().Length);
    }

    [Fact]
    public void Revisions_and_timestamps_reject_invalid_values()
    {
        Assert.Equal(2UL, Revision.Initial.Next().Value);
        Assert.Throws<DomainException>(() => new Revision(0));
        Assert.Throws<DomainException>(() => new UtcMillis(-1));
    }

    [Fact]
    public void Entity_id_rejects_non_v7_guids()
    {
        Assert.Throws<DomainException>(() => EntityId.Parse(Guid.NewGuid().ToString()));
    }
}
