# Bubbles video editor

#### TODO:
* add description basic functionalities
* add video example
* add link to opencv binding + explanation
* The smart pointers/Box<dyn VideoRenderer> "PlayMode" and "PauseMode" are quite large. One should replace them with stack allocated objects 
and use &mut dyn VideoRenderer for dynamic dispatch. This would require a transition function to smootly move data from one state to the other.
* ideally one should be able to "cut" the video to the desired length using a "position selector"