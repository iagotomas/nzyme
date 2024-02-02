package app.nzyme.core.floorplans.db;

import org.jdbi.v3.core.mapper.RowMapper;
import org.jdbi.v3.core.statement.StatementContext;
import org.joda.time.DateTime;

import java.sql.ResultSet;
import java.sql.SQLException;
import java.util.UUID;

public class TenantLocationFloorEntryMapper implements RowMapper<TenantLocationFloorEntry> {

    @Override
    public TenantLocationFloorEntry map(ResultSet rs, StatementContext ctx) throws SQLException {
        return TenantLocationFloorEntry.create(
                rs.getLong("id"),
                UUID.fromString(rs.getString("uuid")),
                UUID.fromString(rs.getString("location_id")),
                rs.getLong("number"),
                rs.getString("name"),
                rs.getBytes("plan"),
                rs.getLong("plan_width"),
                rs.getLong("plan_height"),
                new DateTime(rs.getTimestamp("created_at")),
                new DateTime(rs.getTimestamp("updated_at"))
        );
    }

}
