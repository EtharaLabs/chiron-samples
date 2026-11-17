/*
 * MIT License
 *
 * Copyright (c) 2021-2022 zodac.me
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy
 * of this software and associated documentation files (the "Software"), to deal
 * in the Software without restriction, including without limitation the rights
 * to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 * copies of the Software, and to permit persons to whom the Software is
 * furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in all
 * copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 * AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 * SOFTWARE.
 */

package me.zodac.folding.client.java.request;

import java.io.IOException;
import java.net.URI;
import java.net.http.HttpRequest;
import java.net.http.HttpResponse;
import java.time.Month;
import java.time.Year;
import lombok.AccessLevel;
import lombok.AllArgsConstructor;
import me.zodac.folding.api.util.StringUtils;
import me.zodac.folding.rest.api.exception.FoldingRestException;
import me.zodac.folding.rest.api.header.ContentType;
import me.zodac.folding.rest.api.header.RestHeader;
import me.zodac.folding.rest.util.RestUtilConstants;


@AllArgsConstructor(access = AccessLevel.PRIVATE)
public final class HistoricStatsRequestSender {

    private final String historicStatsUrl;

    
    public static HistoricStatsRequestSender createWithUrl(final String foldingUrl) {
        final String historicStatsUrl = foldingUrl + "/historic";
        return new HistoricStatsRequestSender(historicStatsUrl);
    }

    
    public HttpResponse<String> getHourlyUserStats(final int userId, final Year year, final Month month, final int day) throws FoldingRestException {
        return getHourlyStats(HistoricStatsType.USER, userId, year, month, day, null);
    }

    
    public HttpResponse<String> getHourlyUserStats(final int userId, final Year year, final Month month, final int day, final String entityTag)
        throws FoldingRestException {
        return getHourlyStats(HistoricStatsType.USER, userId, year, month, day, entityTag);
    }

    
    public HttpResponse<String> getHourlyTeamStats(final int teamId, final Year year, final Month month, final int day) throws FoldingRestException {
        return getHourlyStats(HistoricStatsType.TEAM, teamId, year, month, day, null);
    }

    
    public HttpResponse<String> getHourlyTeamStats(final int teamId, final Year year, final Month month, final int day, final String entityTag)
        throws FoldingRestException {
        return getHourlyStats(HistoricStatsType.TEAM, teamId, year, month, day, entityTag);
    }

    private HttpResponse<String> getHourlyStats(final HistoricStatsType historicStatsType, final int id, final Year year, final Month month,
                                                final int day, final String entityTag) throws FoldingRestException {
        final HttpRequest.Builder requestBuilder = HttpRequest.newBuilder()
            .GET()
            .uri(URI.create(
                historicStatsUrl + '/' + historicStatsType.endpointUrl + '/' + id + '/' + year.getValue() + '/' + month.getValue() + '/' + day))
            .header(RestHeader.CONTENT_TYPE.headerName(), ContentType.JSON.contentType());

        if (StringUtils.isNotBlank(entityTag)) {
            requestBuilder.header(RestHeader.IF_NONE_MATCH.headerName(), entityTag);
        }

        final HttpRequest request = requestBuilder.build();

        try {
            return RestUtilConstants.HTTP_CLIENT.send(request, HttpResponse.BodyHandlers.ofString());
        } catch (final InterruptedException e) {
            Thread.currentThread().interrupt();
            throw new FoldingRestException("Error sending HTTP request to get hourly stats for " + historicStatsType.endpointUrl, e);
        } catch (final IOException e) {
            throw new FoldingRestException("Error sending HTTP request to get hourly stats for " + historicStatsType.endpointUrl, e);
        }
    }

    
    public HttpResponse<String> getDailyUserStats(final int userId, final Year year, final Month month) throws FoldingRestException {
        return getDailyStats(HistoricStatsType.USER, userId, year, month, null);
    }

    
    public HttpResponse<String> getDailyUserStats(final int userId, final Year year, final Month month, final String entityTag)
        throws FoldingRestException {
        return getDailyStats(HistoricStatsType.USER, userId, year, month, entityTag);
    }

    
    public HttpResponse<String> getDailyTeamStats(final int teamId, final Year year, final Month month) throws FoldingRestException {
        return getDailyStats(HistoricStatsType.TEAM, teamId, year, month, null);
    }

    
    public HttpResponse<String> getDailyTeamStats(final int teamId, final Year year, final Month month, final String entityTag)
        throws FoldingRestException {
        return getDailyStats(HistoricStatsType.TEAM, teamId, year, month, entityTag);
    }

    private HttpResponse<String> getDailyStats(final HistoricStatsType historicStatsType, final int id, final Year year, final Month month,
                                               final String entityTag) throws FoldingRestException {
        final HttpRequest.Builder requestBuilder = HttpRequest.newBuilder()
            .GET()
            .uri(URI.create(historicStatsUrl + '/' + historicStatsType.endpointUrl + '/' + id + '/' + year.getValue() + '/' + month.getValue()))
            .header(RestHeader.CONTENT_TYPE.headerName(), ContentType.JSON.contentType());

        if (StringUtils.isNotBlank(entityTag)) {
            requestBuilder.header(RestHeader.IF_NONE_MATCH.headerName(), entityTag);
        }

        final HttpRequest request = requestBuilder.build();

        try {
            return RestUtilConstants.HTTP_CLIENT.send(request, HttpResponse.BodyHandlers.ofString());

        } catch (final InterruptedException e) {
            Thread.currentThread().interrupt();
            throw new FoldingRestException("Error sending HTTP request to get daily stats for " + historicStatsType.endpointUrl, e);
        } catch (final IOException e) {
            throw new FoldingRestException("Error sending HTTP request to get daily stats for " + historicStatsType.endpointUrl, e);
        }
    }

    
    public HttpResponse<String> getMonthlyUserStats(final int userId, final Year year) throws FoldingRestException {
        return getMonthlyStats(HistoricStatsType.USER, userId, year, null);
    }

    
    public HttpResponse<String> getMonthlyUserStats(final int userId, final Year year, final String entityTag) throws FoldingRestException {
        return getMonthlyStats(HistoricStatsType.USER, userId, year, entityTag);
    }

    
    public HttpResponse<String> getMonthlyTeamStats(final int teamId, final Year year) throws FoldingRestException {
        return getMonthlyStats(HistoricStatsType.TEAM, teamId, year, null);
    }

    
    public HttpResponse<String> getMonthlyTeamStats(final int teamId, final Year year, final String entityTag) throws FoldingRestException {
        return getMonthlyStats(HistoricStatsType.TEAM, teamId, year, entityTag);
    }

    private HttpResponse<String> getMonthlyStats(final HistoricStatsType historicStatsType, final int id, final Year year, final String entityTag)
        throws FoldingRestException {
        final HttpRequest.Builder requestBuilder = HttpRequest.newBuilder()
            .GET()
            .uri(URI.create(historicStatsUrl + '/' + historicStatsType.endpointUrl + '/' + id + '/' + year.getValue()))
            .header(RestHeader.CONTENT_TYPE.headerName(), ContentType.JSON.contentType());

        if (StringUtils.isNotBlank(entityTag)) {
            requestBuilder.header(RestHeader.IF_NONE_MATCH.headerName(), entityTag);
        }

        final HttpRequest request = requestBuilder.build();

        try {
            return RestUtilConstants.HTTP_CLIENT.send(request, HttpResponse.BodyHandlers.ofString());
        } catch (final InterruptedException e) {
            Thread.currentThread().interrupt();
            throw new FoldingRestException("Error sending HTTP request to get monthly stats for " + historicStatsType.endpointUrl, e);
        } catch (final IOException e) {
            throw new FoldingRestException("Error sending HTTP request to get monthly stats for " + historicStatsType.endpointUrl, e);
        }
    }

    
    private enum HistoricStatsType {

        
        TEAM("teams"),

        
        USER("users");

        private final String endpointUrl;

        
        HistoricStatsType(final String endpointUrl) {
            this.endpointUrl = endpointUrl;
        }
    }
}
