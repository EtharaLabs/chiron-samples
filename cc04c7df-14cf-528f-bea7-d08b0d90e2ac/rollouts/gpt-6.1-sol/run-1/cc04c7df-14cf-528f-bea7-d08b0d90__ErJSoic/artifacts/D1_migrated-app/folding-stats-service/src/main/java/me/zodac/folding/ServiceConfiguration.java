package me.zodac.folding;

import com.zaxxer.hikari.HikariDataSource;
import com.zaxxer.hikari.HikariConfig;
import me.zodac.folding.api.db.DbManager;
import me.zodac.folding.api.stats.FoldingStatsRetriever;
import me.zodac.folding.db.DatabaseType;
import me.zodac.folding.db.postgres.PostgresDbManager;
import me.zodac.folding.stats.HttpFoldingStatsRetriever;
import org.springframework.context.annotation.Bean;
import org.springframework.context.annotation.Configuration;
import org.springframework.core.env.Environment;

@Configuration
public class ServiceConfiguration {
    @Bean(destroyMethod = "close")
    public HikariDataSource dataSource(Environment environment) {
        if (DatabaseType.get(environment.getRequiredProperty("DEPLOYED_DATABASE")) != DatabaseType.POSTGRESQL)
            throw new IllegalStateException("Unsupported DEPLOYED_DATABASE");
        HikariConfig config = new HikariConfig();
        config.setJdbcUrl(environment.getRequiredProperty("JDBC_CONNECTION_URL"));
        config.setUsername(environment.getRequiredProperty("JDBC_CONNECTION_USER"));
        config.setPassword(environment.getRequiredProperty("JDBC_CONNECTION_PASSWORD"));
        config.setDriverClassName(environment.getRequiredProperty("JDBC_CONNECTION_DRIVER"));
        config.addDataSourceProperty("cachePrepStmts", "true");
        config.addDataSourceProperty("prepStmtCacheSize", "250");
        config.addDataSourceProperty("prepStmtCacheSqlLimit", "2048");
        return new HikariDataSource(config);
    }
    @Bean public DbManager dbManager(HikariDataSource source) { return PostgresDbManager.create(source); }
    @Bean public FoldingStatsRetriever foldingStatsRetriever() { return HttpFoldingStatsRetriever.create(); }
}
