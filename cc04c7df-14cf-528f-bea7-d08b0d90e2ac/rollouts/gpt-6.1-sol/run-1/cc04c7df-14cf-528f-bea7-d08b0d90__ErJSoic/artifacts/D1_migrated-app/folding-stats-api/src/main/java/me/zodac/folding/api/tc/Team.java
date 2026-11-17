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

package me.zodac.folding.api.tc;

import java.util.Objects;
import lombok.AccessLevel;
import lombok.AllArgsConstructor;
import lombok.Builder;
import lombok.EqualsAndHashCode;
import lombok.Getter;
import lombok.ToString;
import me.zodac.folding.api.ResponsePojo;
import me.zodac.folding.rest.api.tc.request.TeamRequest;


@AllArgsConstructor(access = AccessLevel.PRIVATE)
@Builder
@Getter
@EqualsAndHashCode
@ToString(doNotUseGetters = true)
public class Team implements ResponsePojo {

    
    public static final int EMPTY_TEAM_ID = 0;

    private final int id;
    private final String teamName;
    private final String teamDescription;
    private final String forumLink;

    
    public static Team create(final int teamId, final String teamName, final String teamDescription, final String forumLink) {
        final String teamDescriptionOrNull = isBlank(teamDescription) ? null : teamDescription;
        final String forumLinkOrNull = isBlank(forumLink) ? null : forumLink;
        return new Team(teamId, teamName, teamDescriptionOrNull, forumLinkOrNull);
    }

    
    public static Team createWithoutId(final String teamName, final String teamDescription, final String forumLink) {
        return create(EMPTY_TEAM_ID, teamName, teamDescription, forumLink);
    }

    
    public static Team createWithoutId(final TeamRequest teamRequest) {
        return createWithoutId(teamRequest.getTeamName(), teamRequest.getTeamDescription(), teamRequest.getForumLink());
    }

    
    public static Team updateWithId(final int teamId, final Team team) {
        return create(teamId, team.teamName, team.teamDescription, team.forumLink);
    }

    private static boolean isBlank(final String input) {
        return input == null || input.isBlank();
    }

    
    public boolean isEqualRequest(final TeamRequest teamRequest) {
        return Objects.equals(teamName, teamRequest.getTeamName())
            && Objects.equals(teamDescription, teamRequest.getTeamDescription())
            && Objects.equals(forumLink, teamRequest.getForumLink());
    }
}
