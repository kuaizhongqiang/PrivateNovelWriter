
#region Project
public class Project
{
    Novel[] novels;

    DetailSample[] details;      // 一些写作细节的样例，为了确定文风，因为可以共用，所以单独拿出来
}

public class Novel
{
    string id;
    string name;
    DateTime created;
    DateTime modified;
    bool active;

    NovelConfig config;               // 小说配置

    NovelSetting setting;             // 小说设定
    Character Actor;                  // 男主唯一
    Character[] Actresses;            // 女主若干
    Character[] otherCharacters;      // 其他角色
    Plugin plugin;                    // 系统
    Outline outline;                   // 大纲
    NovelText text;                    // 正文

    DetailSample[] detailsUesd;       // 使用的写作细节
}

public class DetailSample
{
    string title;
    string content;
}

public class NovelConfig
{
    int totalChar;
    int chapterChar;
    Sensitivity sensitivity = Sensitivity.Normal;
}

public enum Sensitivity
{
    Normal,          // 正常
    Mixed,           // 轻微
    Porn,            // 纯肉
}

public enum NovelType
{
    都市,
    玄幻,
    历史,
    奇幻,
    武侠,
    科幻,
}
#endregion

#region Plugin
// 系统
public class Plugin
{
    string name;                   // 系统名称
    PluginType pluginType;         // 系统类型
    string description;            // 系统描述
    string benifit;                // 好处
    string cost;                   // 代价
}

public enum PluginType
{
    System,
    Gift,
    Prop,
    Skill,
}
#endregion

#region Setting
public class NovelSetting
{
    string title;
    string inspiration;
    string description;
    NovelType type;
    string[] tags;
}

public class Character
{
    string name;
    CharacterType characterType;
    int age;
    string relationshipToActor;
}

public enum CharacterType
{
    Actor,
    Actress,
    Other,
}
#endregion

#region Outline
public class Outline
{
    string worldView;
    PhaseOutline[] phases;
}

public class PhaseOutline
{
    string name;
    string description;
    ChapterOutline[] chapters;
}

public class ChapterOutline
{
    string chapterName;
    string chapterId;
    string content;
    string hook;

}
#endregion

#region Text
public class NovelText
{
    NovelPhase[] phases;
}

public class NovelPhase
{
    int sort;
    string name;
    NovelChapter[] chapters;
}

public class NovelChapter
{
    string id;
    int sort;
    string name;
    string content;
    string path;
}

#endregion